use std::fs::File;
use xml::common::XmlVersion;
use xml::writer::{EmitterConfig, XmlEvent};

pub struct XmlWriter {
    wr_buf: xml::EventWriter<File>,
}

impl XmlWriter {
    pub fn new(file: File) -> XmlWriter {
        let mut xml_writer = XmlWriter {
            wr_buf: EmitterConfig::new()
                .perform_indent(true)
                .create_writer(file),
        };
        xml_writer.wr_buf.write(XmlEvent::StartDocument {
            version: XmlVersion::Version10,
            standalone: None,
            encoding: Some("UTF-8"),
        });
        xml_writer
    }

    pub fn write_element(&mut self, key: String, val: String) {
        self.wr_buf.write(XmlEvent::start_element(key.as_str()));
        self.wr_buf.write(XmlEvent::characters(val.as_str()));
        self.wr_buf.write(XmlEvent::end_element());
    }

    pub fn open_element(&mut self, key: String) {
        self.wr_buf.write(XmlEvent::start_element(key.as_str()));
    }

    pub fn open_element_attr(&mut self, key: String, attr_key: String, attr_val: String) {
        self.wr_buf.write(
            XmlEvent::start_element(key.as_str()).attr(attr_key.as_str(), attr_val.as_str()),
        );
    }

    pub fn close_element(&mut self) {
        self.wr_buf.write(XmlEvent::end_element());
    }

    pub fn comment(&mut self, st: String) {
        self.wr_buf.write(XmlEvent::comment(&st));
    }
}
