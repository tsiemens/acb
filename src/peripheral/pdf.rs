use lopdf::Document;

use crate::util::rw::StringBuffer;

/// page_num should be one-based
pub fn parseable_page_marker(page_num: u32) -> String {
    format!("PAGE_BREAK<{page_num}>")
}

pub fn get_page_marker_pattern() -> regex::Regex {
    regex::Regex::new(r"PAGE_BREAK<(\d+)>").unwrap()
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

pub fn write_doc_text(doc: &Document, w: &mut dyn std::io::Write)
 -> Result<(), pdf_extract::OutputError> {
    let mut output = pdf_extract::PlainTextOutput::new(w);
    pdf_extract::output_doc(doc, &mut output)
}

pub fn write_page_text(doc: &Document, page: u32, w: &mut dyn std::io::Write)
    -> Result<(), pdf_extract::OutputError> {
    let mut copy = doc.clone();
    filter_pdf_pages(&mut copy, &[page]);
    write_doc_text(&copy, w)
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