use std::{collections::VecDeque, rc::Rc, sync::Arc};

use lopdf::Document;

use crate::util::{basic::SError, rw::StringBuffer};

/// page_num should be one-based
pub fn parseable_page_marker(page_num: u32) -> String {
    format!("PAGE_BREAK<{page_num}>")
}

pub fn get_page_marker_pattern() -> regex::Regex {
    regex::Regex::new(r"PAGE_BREAK<(\d+)>").unwrap()
}

pub fn get_num_pages(doc: &Document) -> u32 {
    let mut n_pages = 0;
    for _ in doc.page_iter() {
        n_pages += 1;
    }
    n_pages
}

/// Remove pages not in page_numbers_to_keep.
/// These page numbers should be 1-based.
pub fn filter_pdf_pages(doc: &mut Document, page_numbers_to_keep: &[u32]) -> (u32, u32) {
    let mut total_pages = 0;
    let mut pages_remaining = 0;

    let mut undesired_page_numbers = Vec::<u32>::new();
    for (i, _) in doc.page_iter().enumerate() {
        total_pages += 1;
        let page_num: u32 = (i + 1).try_into().unwrap();
        if !page_numbers_to_keep.contains(&page_num) {
            undesired_page_numbers.push(page_num);
        } else {
            pages_remaining += 1;
        }
    }

    doc.delete_pages(&undesired_page_numbers);
    (total_pages, pages_remaining)
}

struct StringHolder {
    strings: Vec<Option<String>>,
}

struct StringHolderIter<'a> {
    string_holder: &'a StringHolder,
    i: usize,
}

impl<'a> Iterator for StringHolderIter<'a> {
    type Item = (usize, &'a String);
    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.string_holder.strings.len() {
            None
        } else {
            self.i += 1;
            Some((1, &self.string_holder.strings[self.i - 1].as_ref().unwrap()))
        }
    }
}

pub struct LazyPageTextVec {
    doc: Arc<Document>,
    // Contains loaded text. Indexed by page_number - 1

    // This is an Rc<String> so we can pass the ref from the iterator.
    // Directly using &String from the iterator isn't allowed because the iterator
    // takes LazyPageTextVec by &mut
    page_texts: Vec<Option<Rc<String>>>,

    // Caches the last error that was emitted. Mainly for use with iterator.
    pub last_error: Option<SError>,
}

impl LazyPageTextVec {
    pub fn new(doc: Arc<Document>) -> Self {
        Self {
            doc,
            page_texts: Vec::new(),
            last_error: None,
        }
    }

    pub fn load_pages(&mut self, page_numbers: &[u32]) -> Result<(), &SError> {
        let page_texts_res = get_pages_text_arc(self.doc.clone(), &page_numbers);
        match page_texts_res {
            Ok(page_texts) => {
                for (page_num, text) in page_numbers.iter().zip(page_texts) {
                    let page_num_as_index: usize = (page_num - 1) as usize;
                    self.page_texts.resize(page_num_as_index + 1, None);
                    self.page_texts[page_num_as_index] = Some(Rc::new(text));
                }
                Ok(())
            },
            Err(e) => {
                self.last_error = Some(e.to_string());
                // End the iterator
                return Err(self.last_error.as_ref().unwrap());
            },
        }
    }

    pub fn optimized_iter(&mut self,
                          page_groups: Vec<Vec<u32>>) -> OptimizedPageIter {
        OptimizedPageIter::new(self, page_groups)
    }

    /// Takes ideal_page_groups, sanitizes bad pages (out of range), and
    /// appends a final group with all missing pages.
    /// This allows a user to specify what they think they should optimize, but
    /// not need to consider the specifics too much.
    /// The iterator will itself trace which chunks end up being iterated for
    /// debugging of how good the optimization is.
    pub fn safe_page_chunks_with_remainder_pn(
        num_pages: u32,
        ideal_page_groups: &Vec<Vec<u32>>) -> Vec<Vec<u32>> {

        let mut safe_pages = Vec::with_capacity(ideal_page_groups.len() + 1);
        let mut found_pages = std::collections::HashSet::<u32>::new();

        for chunk in ideal_page_groups {
            let mut safe_chunk = Vec::<u32>::with_capacity(chunk.len());
            for page_num in chunk {
                if *page_num <= num_pages && *page_num > 0 {
                    safe_chunk.push(*page_num);
                    found_pages.insert(*page_num);
                } else {
                    tracing::info!("safe_page_chunks_with_remainder pruned out-of-\
                                   range page {page_num}");
                }
            }
            if safe_chunk.len() > 0 {
                safe_pages.push(safe_chunk);
            }
        }

        if found_pages.len() != (num_pages as usize) {
            let extra_chunk: Vec<u32> = (1..num_pages + 1)
                .filter(|p| !found_pages.contains(p))
                .collect();
            safe_pages.push(extra_chunk);
        }

        safe_pages
    }

    /// See `LazyPageTextVec::safe_page_chunks_with_remainder_pn`
    pub fn safe_page_chunks_with_remainder(
        doc: &Document,
        ideal_page_groups: &Vec<Vec<u32>>) -> Vec<Vec<u32>> {
        Self::safe_page_chunks_with_remainder_pn(
            get_num_pages(doc), ideal_page_groups)
    }
}

/// This iterator will load and parse pages in an optimized order specified to
/// it. Since it can take quite a while to parse any given page, if we have a long
/// document where we have a reasonable guess as to which page(s) have the data we're
/// interested in parsing, we would want to load the text for those first.
///
/// Iterating this iterator will load/parse the pages and add them into
/// the LazyPageTextVec, in the order specified by page_groups. Chunks of groups
/// will be loaded in parallel for additional speed.
pub struct OptimizedPageIter<'a> {
    lazy_pages: &'a mut LazyPageTextVec,
    page_groups: Vec<Vec<u32>>,

    next_group: usize,
    // Page number. Every page here will have already been loaded.
    unyielded_pages: VecDeque<u32>,
}

impl <'a> OptimizedPageIter<'a> {
    pub fn new(v: &'a mut LazyPageTextVec, page_groups: Vec<Vec<u32>>) -> Self {
        Self {
            lazy_pages: v,
            page_groups: page_groups,
            next_group: 0,
            unyielded_pages: VecDeque::new(),
        }
    }
}

impl <'a> Iterator for OptimizedPageIter<'a> {
    // Page number and text
    type Item = (u32, Rc<String>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.unyielded_pages.is_empty() {
            if self.next_group >= self.page_groups.len() {
                // Done
                return None;
            } else {
                // Load next group
                if self.next_group == 0 {
                    tracing::trace!("OptimizedPageIter loading group {}",
                                    self.next_group);
                } else {
                    tracing::info!("OptimizedPageIter loading less-optimal group {}",
                                    self.next_group);
                }
                let group_pages = &self.page_groups[self.next_group];
                if self.lazy_pages.load_pages(group_pages).is_err() {
                    // End the iterator
                    return None;
                }

                self.unyielded_pages = group_pages.iter().map(|pn| *pn).collect();
                self.next_group += 1;
            }
        }

        // We want to fail here if we're empty, so unwrap and re-wrap.
        let next_page_num = self.unyielded_pages.pop_front().unwrap();
        let next_idx = (next_page_num - 1) as usize;
        // let next_page: &'a String = self.lazy_pages.page_texts[next_idx].as_ref().unwrap();
        let next_page: Rc<String> = self.lazy_pages.page_texts[next_idx].clone().unwrap();

        Some((next_page_num, next_page))
    }
}

pub fn write_doc_text(doc: &Document, w: &mut dyn std::io::Write)
 -> Result<(), pdf_extract::OutputError> {
    let mut output = pdf_extract::PlainTextOutput::new(w);
    pdf_extract::output_doc(doc, &mut output)
}

pub fn write_page_text(doc: &Document, page: u32, w: &mut dyn std::io::Write)
    -> Result<(), pdf_extract::OutputError> {
    // Note that though this is not that efficient (we will essentially end up
    // creating a copy of the document for every page in it), given how
    // pdf_extract works, this may be the only way apart from re-implementing part
    // of it.
    // This being said, parsing the PDF is by far more expensive than creating these
    // copies, so it's maybe not that much to worry about.
    let start = std::time::Instant::now();
    let mut copy = doc.clone();
    filter_pdf_pages(&mut copy, &[page]);
    let res = write_doc_text(&copy, w);
    tracing::trace!("write_page_text for page {} took {:?}", page, start.elapsed());
    res
}

pub fn get_doc_text(doc: &Document) -> Result<String, pdf_extract::OutputError> {
    let mut buf = StringBuffer::new();
    write_doc_text(doc, &mut buf)?;
    Ok(buf.export_string())
}

pub fn get_page_text(doc: &Document, page: u32)
-> Result<String, pdf_extract::OutputError> {
    let mut buf = StringBuffer::new();
    write_page_text(doc, page, &mut buf)?;
    Ok(buf.export_string())
}

pub async fn get_pages_text_async(doc: Arc<Document>, page_numbers: &[u32])
-> Result<Vec<String>, pdf_extract::OutputError> {
    let start = std::time::Instant::now();

    let mut handles = vec![];

    for page_num in page_numbers {
        let pn_copy = *page_num;
        let arc_clone = doc.clone();
        let handle = async_std::task::spawn(async move {
            get_page_text(arc_clone.as_ref(), pn_copy)
        });
        handles.push(handle);
    }

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        results.push(handle.await);
    }

    tracing::debug!("get_all_pages_text_async took {:?}", start.elapsed());

    let mut pages = Vec::with_capacity(results.len());
    for result in results {
        match result {
            Ok(page) => { pages.push(page); },
            Err(e) => { return Err(e); },
        }
    }

    Ok(pages)
}

pub fn get_pages_text_arc(doc: Arc<Document>, page_numbers: &[u32])
-> Result<Vec<String>, pdf_extract::OutputError> {
    let pages =
        async_std::task::block_on(get_pages_text_async(doc, page_numbers))?;
    Ok(pages)
}

pub fn get_pages_text(doc: Document, page_numbers: &[u32])
-> Result<Vec<String>, pdf_extract::OutputError> {
    get_pages_text_arc(Arc::new(doc), page_numbers)
}

pub async fn get_all_pages_text_async(doc: Document)
-> Result<Vec<String>, pdf_extract::OutputError> {
    let page_numbers = Vec::<u32>::from_iter(
        (0..get_num_pages(&doc)).map(|i| i + 1)
    );
    get_pages_text_async(Arc::new(doc), &page_numbers).await
}

pub fn get_all_pages_text(doc: Document)
-> Result<Vec<String>, pdf_extract::OutputError> {
    let pages =
        async_std::task::block_on(get_all_pages_text_async(doc.clone()))?;
    Ok(pages)
}

pub fn get_all_pages_text_from_path(p: &std::path::PathBuf)
-> Result<Vec<String>, crate::util::basic::SError> {
    let doc = Document::load(p).map_err(|e| e.to_string())?;
    get_all_pages_text(doc).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use crate::testlib::assert_vec_eq;

    use super::LazyPageTextVec;

    #[test]
    fn test_safe_page_chunks_with_remainder_pn() {
        let safe_chunks = LazyPageTextVec::safe_page_chunks_with_remainder_pn(
            1, &vec![vec![0]]
        );
        assert_vec_eq(safe_chunks, vec![vec![1]]);

        let safe_chunks = LazyPageTextVec::safe_page_chunks_with_remainder_pn(
            1, &vec![vec![1]]
        );
        assert_vec_eq(safe_chunks, vec![vec![1]]);

        let safe_chunks = LazyPageTextVec::safe_page_chunks_with_remainder_pn(
            1, &vec![vec![2]]
        );
        assert_vec_eq(safe_chunks, vec![vec![1]]);

        let safe_chunks = LazyPageTextVec::safe_page_chunks_with_remainder_pn(
            4, &vec![]
        );
        assert_vec_eq(safe_chunks, vec![vec![1, 2, 3, 4]]);

        let safe_chunks = LazyPageTextVec::safe_page_chunks_with_remainder_pn(
            4, &vec![vec![1, 3], vec![4]]
        );
        assert_vec_eq(safe_chunks, vec![vec![1, 3], vec![4], vec![2]]);

        let safe_chunks = LazyPageTextVec::safe_page_chunks_with_remainder_pn(
            4, &vec![vec![1, 3, 5], vec![4, 2]]
        );
        assert_vec_eq(safe_chunks, vec![vec![1, 3], vec![4, 2]]);
    }
}