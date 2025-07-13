// A slightly simplified wrapper around the zip writer, for writing to a byte buffer.
// Only supports Store compression (uncompressed) right now, since we are not
// as interested in compression, but rather to bundle multiple csv files together.
pub struct ZipWriter {
    // output: RcRefCell<Vec<u8>>,
    archive: rawzip::ZipArchiveWriter<Vec<u8>>,
}

pub struct ZipEntryWriter<'a> {
    entry_writer: rawzip::ZipEntryWriter<'a, Vec<u8>>,
}

impl<'a> ZipEntryWriter<'a> {
    pub fn new(entry_writer: rawzip::ZipEntryWriter<'a, Vec<u8>>) -> Self {
        ZipEntryWriter { entry_writer }
    }

    pub fn create_data_writer<'b>(
        &'b mut self,
    ) -> rawzip::ZipDataWriter<&'b mut rawzip::ZipEntryWriter<'a, Vec<u8>>> {
        rawzip::ZipDataWriter::new(&mut self.entry_writer)
    }

    pub fn finish(
        self,
        descriptor: rawzip::DataDescriptorOutput,
    ) -> Result<u64, String> {
        self.entry_writer
            .finish(descriptor)
            .map_err(|e| format!("Failed to finish zip entry writer: {}", e))
    }
}

impl ZipWriter {
    pub fn new() -> ZipWriter {
        ZipWriter {
            archive: rawzip::ZipArchiveWriter::new(Vec::new()),
        }
    }

    pub fn start_file<'b>(
        &'b mut self,
        name: &str,
    ) -> Result<ZipEntryWriter<'b>, String> {
        // We are not compressing the data, so we can use the Store compression method.
        // Note that at the time of writing, this is the only entry option available
        // in rawzip. Default file permission seems to be o664 (rw-rw-r--),
        // which is fine. Unfortunately though the modified date is set to 0,
        // so they all end up showing up as Dec 31 1979.

        // Start of a new file in our zip archive.
        let entry_writer = self.archive.new_file(name)
            .compression_method(rawzip::CompressionMethod::Store)
            .create()
            .map_err(|e| {
                format!("Failed to create new file in zip archive: {}", e)
            })?;

        // We're not doing any compression so we can just use the raw file.
        // No need to wrap in an encoder.
        Ok(ZipEntryWriter::new(entry_writer))
    }

    pub fn finish(self) -> Result<Vec<u8>, String> {
        Ok(self
            .archive
            .finish()
            .map_err(|e| format!("Failed to finish zip archive: {}", e))?)
    }
}

#[cfg(test)]
mod tests {
    fn do_zipwriter_test() -> Result<(), String> {
        let mut zip_writer = super::ZipWriter::new();

        let mut entry_writer = zip_writer.start_file("file.txt")?;

        let mut data_writer = entry_writer.create_data_writer();

        let data = b"Hello, world!";
        std::io::copy(&mut &data[..], &mut data_writer)
            .map_err(|e| format!("Failed to write data to zip file: {}", e))?;

        let (_, descriptor) = data_writer
            .finish()
            .map_err(|e| format!("Failed to finish zip data writer: {}", e))?;
        let compressed_size = entry_writer.finish(descriptor)?;

        assert_ne!(
            compressed_size, 0,
            "Compressed size should be greater than 0"
        );

        // Finish the zip archive and get the output bytes.
        let output = zip_writer.finish()?;

        // Read back what was written.
        let archive = rawzip::ZipArchive::from_slice(&output).map_err(|e| {
            format!("Failed to create zip archive from slice: {}", e)
        })?;

        // Rawzip does not materialize the central directory when a Zip archive is parsed,
        // so we need to iterate over the entries to find the one we want.
        let mut entries = archive.entries();

        // Get the first (and only) entry in the archive.
        let entry = entries
            .next_entry()
            .map_err(|e| format!("No next entry: {e}"))?
            .unwrap();

        assert_eq!(entry.file_path().try_normalize().unwrap().as_ref(), "file.txt");

        assert_eq!(entry.compression_method(), rawzip::CompressionMethod::Store);

        // Assert the uncompressed size hint. Be warned that this may not be the actual,
        // uncompressed size for malicious or corrupted files.
        assert_eq!(entry.uncompressed_size_hint(), data.len() as u64);

        // Before we need to access the entry's data, we need to know where it is in the archive.
        let wayfinder = entry.wayfinder();

        let local_entry = archive
            .get_entry(wayfinder)
            .map_err(|e| format!("Failed to get entry from archive: {}", e))?;

        let mut actual = Vec::new();
        // There is no compression, so we can just use the raw data directly.
        let decompressor = local_entry.data();

        assert_eq!(decompressor, data);

        // We wrap the decompressor in a verifying reader, which will verify the size and CRC of
        // the decompressed data once finished.
        let mut reader = local_entry.verifying_reader(decompressor);
        std::io::copy(&mut reader, &mut actual)
            .map_err(|e| format!("Failed to read data from zip entry: {}", e))?;

        // Assert the data is what we wrote.
        assert_eq!(&data[..], actual);
        Ok(())
    }

    #[test]
    fn test_zipwriter() {
        assert!(do_zipwriter_test().is_ok());
    }
}
