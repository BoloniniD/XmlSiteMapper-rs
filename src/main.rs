use chrono::{SecondsFormat, Utc};
use regex::Regex;
use reqwest::{StatusCode, Url};
use select::document::Document;
use select::predicate::Name;
use std::env;
use std::fs::File;
use std::io::{prelude::*, BufReader, BufWriter, Write};
use std::{collections::HashMap, collections::HashSet, collections::VecDeque, thread};

mod terminal_writer;
use terminal_writer::TermWriter;

mod xml_file_writer;
use xml_file_writer::XmlWriter;

pub struct Mapper {
    main_url: Url,
    disallowed_extensions: HashSet<String>,
    change_prio: HashMap<String, f64>,
    delay: u64,
    term: TermWriter,
}

impl Mapper {
    pub fn new(main_url: Url, disallowed_extensions: HashSet<String>, change_prio: HashMap<String, f64>, delay: u64, terminal: TermWriter) -> Mapper {
        Mapper{main_url, disallowed_extensions, change_prio, delay, term: terminal}
    }

    fn start_logging(&mut self, links: i64, map_len: usize, file_writer: &mut BufWriter<&mut File>) {
        self.term.start_progress(links, map_len);
        match writeln!(
            file_writer,
            "Crawling start: [{}, {}]",
            Utc::now().date(),
            Utc::now().time().format("%H:%M:%S")
        ) {
            Ok(_) => {
                // OK
            },
            Err(_) => {
                // UNABLE TO WRITE LOG
            },
        }
    }

    fn log_progress(&mut self, links: i64, map_len: usize, file_writer: &mut BufWriter<&mut File>) {
        match writeln!(
            file_writer,
            "\nSize of queue on this iteration: {}",
            links
        ) {
            Ok(_) => {
                // OK
            },
            Err(_) => {
                // UNABLE TO WRITE LOG
            }
        }
        self.term.print_progress(links, map_len);
    }

    fn normalize_url(&self, url: Url) -> Option<Url> {
        let norm = url_normalizer::normalize(url);
        match norm {
            Ok(mut norm) => {
                let u = norm.set_scheme(self.main_url.scheme());
                match u {
                    Ok(_) => {
                        Some(norm)
                    },
                    Err(_) => {
                        None
                    }
                }
            }
            Err(_) => {
                None
            }
        }
    }

    fn priority_changes_segment_count(&self, mut priority: f64, url: &Url) -> f64 {
        let seg = url.path_segments();
        match seg {
            Some(seg) => {
                priority -= 0.1 * (seg.count() as f64 - 1.0 + url.query_pairs().count() as f64);
            }
            None => {
                priority -= 0.1 * (url.query_pairs().count() as f64);
            }
        }
        priority
    }

    fn update_map(&self, map: &mut HashMap<Url, f64>, url: &Url, priority: &mut f64, file_writer: &mut BufWriter<&mut File>) {
        if !map.contains_key(&url) {
            let url_str = url.as_str();
            for i in self.change_prio.iter() {
                let re = Regex::new(i.0);
                match re {
                    Ok(re) => {
                        if re.is_match(url_str) {
                            *priority += i.1;
                        }
                    }
                    Err(_) => {
                        writeln!(
                            file_writer,
                            "Error while parsing a regex from disallow.cfg: {}",
                            i.0
                        );
                    }
                }
            }
            if *priority < 0.1 {
                *priority = 0.1;
            }
            map.insert(url.clone(), *priority);
        }
    }

    fn get_body(&self, url: &Url, file_writer: &mut BufWriter<&mut File>) -> Option<reqwest::blocking::Response> {
        let client = reqwest::blocking::Client::new();
        let client = client.get(url.clone()).send();
        let body: reqwest::blocking::Response;
        match client {
            Ok(res) => {
                body = res;
            }
            Err(_) => {
                return None;
            }
        }
        match body.status() {
            StatusCode::OK => {
                writeln!(file_writer, "Successfully pinged '{}'.", url);
                Some(body)
            }
            s => {
                writeln!(file_writer, "Received {} status code, skipping...", s);
                None
            }
        }
    }

    fn check_header(&self, body: &reqwest::blocking::Response) -> bool {
        match body.headers().get("Content-Type") {
            Some(url_check) => {
                match url_check.to_str() {
                    Ok(st) => {
                        if String::from(st).starts_with("text/html") {
                            return true;
                        } else {
                            return false;
                        }
                    }
                    Err(_) => {
                        return false;
                    }
                }
            }
            None => {
                return false;
            }
        }
    }

    fn check_disallowed(&self, link: &str, file_writer: &mut BufWriter<&mut File>) -> bool {
        let mut flag = false;
        for i in self.disallowed_extensions.iter() {
            let re = Regex::new(i);
            match re {
                Ok(re) => {
                    if re.is_match(link) {
                        flag = true;
                    }
                }
                Err(_) => {
                    writeln!(
                        file_writer,
                        "Error while parsing a regex from disallow.cfg: {}",
                        i
                    );
                }
            }
        }
        flag
    }

    fn scan_link(&mut self, map: &mut HashMap<Url, f64>, log: &mut File) {
        let mut file_writer = BufWriter::new(log);
        let mut queue: VecDeque<Url> = VecDeque::new();
        let mut set: HashSet<Url> = HashSet::new();
        let mut links: i64 = 1;
        queue.push_front(self.main_url.clone());
        set.insert(self.main_url.clone());
        self.start_logging(links, map.len(), &mut file_writer);
        while !queue.is_empty() {
            self.log_progress(links, map.len(), &mut file_writer);
            let ten_millis = std::time::Duration::from_millis(self.delay);
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
            match self.normalize_url(url) {
                Some(normalized) => {
                    url = normalized;
                },
                None => {
                    continue;
                }
            }
            if url.domain() != self.main_url.domain() {
                continue;
            }
            writeln!(&mut file_writer, "\nWorking with '{}' now", url.as_str());
            let mut priority: f64 = 1.0;
            priority = self.priority_changes_segment_count(priority, &url);
            let mut body: reqwest::blocking::Response;
            match self.get_body(&url, &mut file_writer) {
                Some(result) => {
                    body = result;
                },
                None => {
                    continue;
                }
            }
            self.update_map(map, &url, &mut priority, &mut file_writer);
            match self.check_header(&body) {
                false => {
                    continue;
                },
                true => {},
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
                                || (link.starts_with(self.main_url.as_str()))
                            {
                                let flag = self.check_disallowed(link, &mut file_writer);
                                let link = self.main_url.join(link).unwrap();
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

    pub fn generate_sitemap(&mut self, mut log: &mut File) -> HashMap<Url, f64> {
        let mut result_map = HashMap::<Url, f64>::new();
        self.scan_link(&mut result_map, &mut log);
        result_map
    }
}

fn read_disallowed_exts(term: &TermWriter, exts: &mut HashSet<String>) {
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
            exts.insert(String::from(".*/.*print=Y"));
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
                    term.print_to_term(format!("This file is used to contain all url parts which should be excluded from sitemap."));
                    term.print_to_term(format!("For example, if .*.pdf is on the list then link 'https://foo.com/bar.pdf' won't be included in the sitemap."));
                    term.print_to_term(format!(
                        "You can also write comments in site.cfg starting the lines with #.",
                    ));
                    let data = "# For example, you can exclude all URLs leading to .png images by writing a regex with it.
# To do this, you should write one line per each file type (note, that mapper is case sensetive, so .png and .PNG are different file types for it)\n# The result should look like next line without '# ':
# .*.png\n# This will make all URLs like https://foo.bar/cool_image.png excluded from sitemap.xml.";
                    std::fs::write("disallow.cfg", data).expect("Unable to write file");
                }
                Err(_) => {
                    term.print_to_term(format!("Unable to create file disallow.cfg."));
                }
            }
        }
    }
}

fn read_priority_changes(term: &TermWriter, chng: &mut HashMap<String, f64>) {
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
                    term.print_to_term(format!("This file is used to show which url containing listed lines should be lowered or increased in priority."));
                    term.print_to_term(format!("For each one the first line should contain the part of url written with regex and the second one should contain the number"));
                    term.print_to_term(format!(
                        "For example 0.5 for raise priority for 0.5 or -0.3 to lower it by 0.3.",
                    ));
                    term.print_to_term(format!(
                        "You can also write comments in site.cfg starting the lines with #.",
                    ));
                    let data = "# For example, you can lower priority for all URLs with PAGEN_1 contained in query.
# To do this, you should write two lines, containing 1) regex containing PAGEN_1 2) priority change\n# The result should look like next two comment lines without '# ':
# .*PAGEN_1.*\n# -0.2\n# This will lower priority for all pages like https://foo.bar/?PAGEN_1=50 by 0.2.";
                    std::fs::write("change_prio.cfg", data).expect("Unable to write file");
                }
                Err(_) => {
                    term.print_to_term(format!("Unable to create file change_prio.cfg."));
                }
            }
        }
    }
}

fn read_site(term: &TermWriter, url: &mut String, delay: &mut u64) {
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
                                *url = line;
                                flag = false;
                            } else {
                                let delay_chk = line.parse::<u64>();
                                match delay_chk {
                                    Ok(d) => *delay = d,
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
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut active_term = true;
    let mut path: Option<String> = None;
    for it in 0..args.len() {
        let arg = args[it].as_str();
        match arg {
            "--help" => {
                println!("[-p <path>] [-s | --silent]");
                return;
            }
            "--silent" => active_term = false,
            "-s" => active_term = false,
            "-p" => match args.get(it + 1) {
                Some(p) => path = Some(String::from(p)),
                None => {
                    path = None;
                    let term = TermWriter::new(true);
                    term.print_to_term(format!("Found key -p which is not followed by a path, assuming path is executable's directory."));
                }
            },
            _ => {}
        }
    }
    let term = TermWriter::new(active_term);
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

    read_disallowed_exts(&term, &mut exts);
    read_priority_changes(&term, &mut chng);

    let mut delay: u64 = 25;
    let mut url = String::new();

    read_site(&term, &mut url, &mut delay);
    
    let url = Url::parse(&url);
    match url {
        Ok(main_url) => {
            term.print_to_term(format!(
                "All necessary files checked, starting sitemap.xml generation."
            ));
            let mut mapper = Mapper::new(main_url, exts, chng, delay, term.clone());
            let map = mapper.generate_sitemap(&mut log);
            match path {
                Some(path) => {
                    final_messg = format!(
                        "sitemap.xml generation is completed. You can find it here: '{}'.",
                        String::from(&path) + "sitemap.xml"
                    );
                    fil = File::create(String::from(&path) + "sitemap.xml")
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
            let mut writer = XmlWriter::new(file);
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
