use std::fs::File;
use std::path::Path;

pub mod terminal_writer;
use terminal_writer::TermWriter::TermWriter;

pub struct FileWriter {
    file: String,
    term: TermWriter,
}

impl FileWriter {
    pub fn new(out_file: String, _term: TermWriter) -> FileWriter {
        let writer = FileWriter{file: out_file.clone(), term: _term};
        if !Path::new(&out_file).exists() {
            match File::create(&out_file) {
                Ok(_) => {},
                Err(_) => {
                    writer.term.print_to_term(format!("Cannot create file {}", out_file));
                }
            }
        } else {}
        writer
    }

    pub fn write_string(line: String) {
        std::fs::write("disallow.cfg", format!("{}\n", line)).expect("Unable to write file");
    }

    pub fn comment_string(&mut self, line: String) {

    }
}
