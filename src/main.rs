use chrono::{SecondsFormat, Utc};
use console::Term;
use reqwest::{StatusCode, Url};
use select::document::Document;
use select::predicate::Name;
use std::env;
use std::fs::File;
use std::io::{prelude::*, BufReader, BufWriter, Write};
use std::{collections::HashMap, collections::HashSet, collections::VecDeque, thread};
use xml::common::XmlVersion;
use xml::writer::{EmitterConfig, XmlEvent};

#[derive(Clone)]
struct term_writer {
    term: Term,
}

impl term_writer {
    pub fn new() -> term_writer {
        term_writer {
            term: Term::stdout(),
        }
    }

    pub fn print_progress(&self, links: i64, total: usize) {
        self.term.clear_last_lines(2);
        self.term
            .write_line(&format!("Size of queue on this iteration: {}", links));
        self.term
            .write_line(&format!("Total links found: {}", total));
    }

    pub fn start_progress(&self, links: i64, total: usize) {
        self.term
            .write_line(&format!("Size of queue on this iteration: {}", links));
        self.term
            .write_line(&format!("Total links found: {}", total));
    }

    pub fn print_to_term(&self, st: String) {
        self.term.write_line(&st);
    }
}

struct xml_writer {
    wr_buf: xml::EventWriter<File>,
}

impl xml_writer {
    pub fn new(file: File) -> xml_writer {
        let mut xml_writer = xml_writer {
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

fn scan_link(
    main_url: Url,
    map: &mut HashMap<Url, f64>,
    exts: HashSet<String>,
    chng: HashMap<String, f64>,
    delay: u64,
    log: &mut File,
    term: &term_writer,
) {
    // TODO: create a class and split it into several methods
    // TODO: write comments
    let mut file_writer = BufWriter::new(log);
    let mut queue: VecDeque<Url> = VecDeque::new();
    let mut set: HashSet<Url> = HashSet::new();
    let mut links: i64 = 1;
    queue.push_front(main_url.clone());
    set.insert(main_url.clone());
    term.start_progress(links, map.len());
    writeln!(
        &mut file_writer,
        "Crawling start: [{}, {}]",
        Utc::now().date(),
        Utc::now().time().format("%H:%M:%S")
    );
    while !queue.is_empty() {
        writeln!(
            &mut file_writer,
            "\nSize of queue on this iteration: {}",
            links
        );
        term.print_progress(links, map.len());
        let ten_millis = std::time::Duration::from_millis(delay);
        thread::sleep(ten_millis);
        let queue_pop = queue.pop_back();
        links -= 1;
        let mut url: Url;
        match queue_pop {
            Some(queue_pop) => {
                url = queue_pop;
            }
            None => {
                continue;
            }
        }
        let norm = url_normalizer::normalize(url);
        match norm {
            Ok(norm) => {
                url = norm;
                let u = url.set_scheme(main_url.scheme());
                match u {
                    Ok(_) => {}
                    Err(_) => {
                        continue;
                    }
                }
            }
            Err(_) => {
                continue;
            }
        }
        if url.domain() != main_url.domain() {
            continue;
        }
        writeln!(&mut file_writer, "\nWorking with '{}' now", url.as_str());
        let seg = url.path_segments();
        let mut priority: f64 = 1.0;
        match seg {
            Some(seg) => {
                priority -= 0.1 * (seg.count() as f64 - 1.0 + url.query_pairs().count() as f64);
            }
            None => {
                priority -= 0.1 * (url.query_pairs().count() as f64);
            }
        }
        if !map.contains_key(&url) {
            let mut query = url.query_pairs();
            loop {
                match query.next() {
                    Some(q) => {
                        for i in chng.iter() {
                            let q = q.0.as_ref();
                            if i.0.contains(q) {
                                priority += i.1;
                            }
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
            if priority < 0.1 {
                priority = 0.1;
            }
            map.insert(url.clone(), priority);
        }
        let client = reqwest::blocking::Client::new();
        let client = client.get(url.clone()).send();
        let body: reqwest::blocking::Response;
        match client {
            Ok(res) => {
                body = res;
            }
            Err(_) => {
                continue;
            }
        }
        match body.status() {
            StatusCode::OK => {
                writeln!(&mut file_writer, "Successfully pinged '{}'.", url);
            }
            s => {
                writeln!(&mut file_writer, "Received {} status code, skipping...", s);
                continue;
            }
        }
        let url_check = body.headers().get("Content-Type");
        match url_check {
            Some(url_check) => {
                let url_check = url_check.to_str();
                match url_check {
                    Ok(st) => {
                        if String::from(st).starts_with("text/html") {
                        } else {
                            *map.get_mut(&url).unwrap() -= 0.1;
                            continue;
                        }
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
            None => {
                continue;
            }
        }
        let body = body.text();
        match body {
            Ok(body) => {
                let html = body;
                let html = Document::from(html.as_str());
                html.find(Name("a"))
                    .filter_map(|h| h.attr("href"))
                    .for_each(|link| {
                        if ((link.starts_with('/')) && (link != "/"))
                            || (link.starts_with(main_url.as_str()))
                        {
                            let mut flag = false;
                            for i in exts.iter() {
                                if link.ends_with(i) {
                                    flag = true;
                                }
                            }
                            let link = main_url.join(link).unwrap();
                            let query_chk = link.query();
                            match query_chk {
                                Some(query) => {
                                    // print versions of pages are mostly equal to regular ones, so we'll skip them
                                    if query.contains("print=Y") {
                                        flag = true;
                                    }
                                }
                                None => {}
                            }
                            if !flag {
                                let link = url_normalizer::normalize(link);
                                match link {
                                    Ok(link) => {
                                        if !set.contains(&link) {
                                            links += 1;
                                            set.insert(link.clone());
                                            queue.push_front(link);
                                        }
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                    });
            }
            Err(_) => {}
        }
    }
    writeln!(
        &mut file_writer,
        "Crawling end: [{}]\nBuilding file sitemap.xml.",
        Utc::now().time().format("%H:%M:%S")
    );
}

fn main() {
    let term = term_writer::new();
    let args: Vec<String> = env::args().collect();
    let fil: std::io::Result<File>;
    let final_messg: String;
    let logger = File::create("XmlSiteMapper-rs.log");
    let file: File;
    let mut log: File;
    match logger {
        Ok(logger) => {
            log = logger;
        }
        Err(_) => {
            term.print_to_term(format!("Cannot create file XmlSiteMapper-rs.log. Please check if file creation is allowed in the directory."));
            return;
        }
    }
    let mut exts: HashSet<String> = HashSet::new();
    let mut chng: HashMap<String, f64> = HashMap::new();

    let disallowed = File::open("disallow.cfg");
    match disallowed {
        Ok(disallowed) => {
            let disallowed = BufReader::new(disallowed);
            for line in disallowed.lines() {
                match line {
                    Ok(line) => {
                        if !line.starts_with("#") {
                            exts.insert(line);
                        }
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
        }
        Err(_) => {
            term.print_to_term(format!(
                "File disallow.cfg was not found, creating one instead."
            ));
            let chk = File::create("disallow.cfg");
            match chk {
                Ok(_) => {
                    term.print_to_term(format!("==="));
                    term.print_to_term(format!("Created file disallow.cfg."));
                    term.print_to_term(format!("This file is used to contain all file extensions which should be excluded from sitemap."));
                    term.print_to_term(format!("For example, if .pdf is on the list then link 'https://foo.com/bar.pdf' won't be included in the sitemap."));
                    term.print_to_term(format!(
                        "You can also write comments in site.cfg starting the lines with #.",
                    ));
                    let data = "# For example, you can exclude all URLs leading to .png images.
# To do this, you should write one line per each file type (note, that mapper is case sensetive, so .png and .PNG are different file types for it)\n# The result should look like next line without '# ':
# .png\n# This will make all URLs like https://foo.bar/cool_image.png excluded from sitemap.xml.";
                    std::fs::write("disallow.cfg", data).expect("Unable to write file");
                }
                Err(_) => {
                    term.print_to_term(format!("Unable to create file disallow.cfg."));
                }
            }
        }
    }

    let to_change = File::open("change_prio.cfg");
    match to_change {
        Ok(to_change) => {
            let to_change = BufReader::new(to_change);
            let mut prev = String::from("#");
            for line in to_change.lines() {
                match line {
                    Ok(line) => {
                        if !line.starts_with("#") && prev == "#" {
                            prev = line;
                        } else if !line.starts_with("#") && prev != "#" {
                            match line.parse::<f64>() {
                                Ok(flt) => {
                                    chng.insert(prev.clone(), flt);
                                }
                                Err(_) => {}
                            }
                            prev = String::from("#");
                        }
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
        }
        Err(_) => {
            term.print_to_term(format!(
                "File change_prio.cfg not found, creating one instead."
            ));
            let chk = File::create("change_prio.cfg");
            match chk {
                Ok(_) => {
                    term.print_to_term(format!("==="));
                    term.print_to_term(format!("Created file change_prio.cfg."));
                    term.print_to_term(format!("This file is used to show which queries in url should be lowered in priority."));
                    term.print_to_term(format!("For each query the first line should contain its name and the second one should contain the number"));
                    term.print_to_term(format!(
                        "For example 0.5 for raise priority for 0.5 or -0.3 to lower it by 0.3.",
                    ));
                    term.print_to_term(format!(
                        "You can also write comments in site.cfg starting the lines with #.",
                    ));
                    let data = "# For example, you can lower priority for all URLs with PAGEN_1 contained in query.
# To do this, you should write two lines, containing 1) query name 2) priority change\n# The result should look like next two comment lines without '# ':
# PAGEN_1\n# -0.2\n# This will lower priority for all pages like https://foo.bar/PAGEN_1=50 by 0.2.";
                    std::fs::write("change_prio.cfg", data).expect("Unable to write file");
                }
                Err(_) => {
                    term.print_to_term(format!("Unable to create file change_prio.cfg."));
                }
            }
        }
    }
    let mut delay: u64 = 25;
    let mut url = String::new();
    let site = File::open("site.cfg");
    match site {
        Ok(site) => {
            let site = BufReader::new(site);
            let mut flag: bool = true;
            for line in site.lines() {
                match line {
                    Ok(line) => {
                        if !line.starts_with("#") {
                            if flag {
                                url = line;
                                flag = false;
                            } else {
                                let delay_chk = line.parse::<u64>();
                                match delay_chk {
                                    Ok(d) => delay = d,
                                    Err(_) => {
                                        continue;
                                    }
                                }
                                break;
                            }
                        }
                    }
                    Err(_) => {
                        if flag {
                            term.print_to_term(format!(
                                "It seems, that sitemapper can't detect an URL in site.cfg.",
                            ));
                            term.print_to_term(format!("Please check if it is correct."));
                        }
                        continue;
                    }
                }
            }
        }
        Err(_) => {
            term.print_to_term(format!("File site.cfg not found, creating one instead."));
            let chk = File::create("site.cfg");
            match chk {
                Ok(_) => {
                    term.print_to_term(format!("==="));
                    term.print_to_term(format!("Created file site.cfg."));
                    term.print_to_term(format!(
                        "Now you can use site.cfg to store URL that is going to be mapped.",
                    ));
                    term.print_to_term(format!("You can specify the delay (in ms) between url requests in the next line. The default is 25 ms."));
                    term.print_to_term(format!(
                        "You can also write comments in site.cfg starting the lines with #.",
                    ));
                    term.print_to_term(format!("\nFill in this file and launch the mapper again."));
                    let data = "# Write your site root URL in the next line. Please, include protocol in the URL (http or https).\n
# Write the delay in milliseconds between URL requests in the next line (this one is optional, but may be useful if your site blocks bots by number of requests per time fragment):\n";
                    std::fs::write("site.cfg", data).expect("Unable to write file");
                    return;
                }
                Err(_) => {
                    term.print_to_term(format!("Unable to create file site.cfg."));
                }
            }
        }
    }
    let url = Url::parse(&url);
    match url {
        Ok(_) => {
            let mut map = HashMap::new();
            term.print_to_term(format!(
                "All necessary files checked, starting sitemap.xml generation."
            ));
            scan_link(url.unwrap(), &mut map, exts, chng, delay, &mut log, &term);
            match args.get(1) {
                Some(path) => {
                    final_messg = format!(
                        "sitemap.xml generation is completed. You can find it here: '{}'.",
                        String::from(path) + "sitemap.xml"
                    );
                    fil = File::create(String::from(path) + "sitemap.xml")
                }
                None => {
                    final_messg = format!(
                        "sitemap.xml generation is completed. You can find it in the same directory with the executable.",
                    );
                    fil = File::create("sitemap.xml");
                }
            }
            match fil {
                Ok(fil) => {
                    file = fil;
                }
                Err(_) => {
                    term.print_to_term(format!("Cannot create file sitemap.xml. Please check if file creation is allowed in the directory."));
                    return;
                }
            }
            term.print_to_term(format!("\n=====\nTotal urls added: {}\n=====", map.len()));
            let mut writer = xml_writer::new(file);
            // TODO: Write a checker for xml writer Result<()>
            writer.comment(String::from("=== Created with XmlSiteMapper-rs ==="));
            writer.open_element_attr(
                String::from("urlset"),
                String::from("xmlns"),
                String::from("http://www.sitemaps.org/schemas/sitemap/0.9"),
            );
            for (key, value) in map {
                writer.open_element(String::from("url"));

                writer.write_element(String::from("loc"), String::from(key.as_str()));

                let now = Utc::now();

                writer.write_element(
                    String::from("lastmod"),
                    String::from(now.to_rfc3339_opts(SecondsFormat::Secs, false)),
                );

                writer.write_element(String::from("priority"), format!("{0:.1}", value));

                writer.close_element();
            }
            writer.close_element();
            term.print_to_term(final_messg);
            let mut file_writer = BufWriter::new(log);
            writeln!(
                &mut file_writer,
                "Built file sitemap.xml: [{}]",
                Utc::now().time().format("%H:%M:%S")
            );
        }
        Err(_) => {
            return;
        }
    }
}
