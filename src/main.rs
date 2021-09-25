use chrono::{SecondsFormat, Utc};
use regex::Regex;
use reqwest::{StatusCode, Url};
use select::document::Document;
use select::predicate::Name;
use std::{collections::HashMap, collections::HashSet, collections::VecDeque, thread, 
    env, fs::File, io::{prelude::*, BufReader, BufWriter, Write}};

mod terminal_writer;
use terminal_writer::TermWriter;

mod xml_file_writer;
use xml_file_writer::XmlWriter;

mod file_writer;
use file_writer::FileWriter;

struct Launcher {
    term: TermWriter,
    file: FileWriter,
}

fn main() {

}