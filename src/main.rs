use reqwest::{StatusCode, Url};
use select::document::Document;
use select::predicate::Name;
use std::fs::File;
use std::io::{prelude::*, BufReader, BufWriter, Write};
use std::{collections::HashMap, collections::HashSet, collections::VecDeque, thread, time};

const DELAY: u64 = 25;

fn scan_link(
    main_url: Url,
    map: &mut HashMap<Url, f64>,
    exts: HashSet<String>,
    chng: HashMap<String, f64>,
) {
    // TODO: create a class and split it into several methods
    let mut queue: VecDeque<Url> = VecDeque::new();
    let mut set: HashSet<Url> = HashSet::new();
    let mut links: i64 = 1;
    queue.push_front(main_url.clone());
    set.insert(main_url.clone());
    while !queue.is_empty() {
        println!("Size of queue: {}", links);
        let ten_millis = time::Duration::from_millis(DELAY);
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
        println!("\nWorking with '{}' now", url.as_str());
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
            let query = url.query();
            match query {
                Some(q) => {
                    for i in chng.iter() {
                        if i.0.contains("PAGEN") {
                            priority += i.1;
                        }
                    }
                }
                None => {}
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
                println!("Sent a request successfully");
                body = res;
            }
            Err(_) => {
                println!("Failed to send a request");
                continue;
            }
        }
        match body.status() {
            StatusCode::OK => println!("Successfully pinged '{}'.", url),
            s => {
                println!("Received {} status code, skipping...", s);
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
                            println!("Parsing html for links...");
                        } else {
                            println!("Page is not html: {}", st);
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
                println!("Page does not contain Content-Type header");
                continue;
            }
        }
        let body = body.text();
        match body {
            Ok(body) => {
                let html = body;
                let html = Document::from(html.as_str());
                println!("Parsing html for...");
                html.find(Name("a"))
                    .filter_map(|h| h.attr("href"))
                    .for_each(|link| {
                        println!("Found '{}'...", link);
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
                        } else {
                            println!("This link is not needed while building a sitemap.")
                        }
                    });
            }
            Err(_) => {}
        }
    }
}

fn main() {
    let file = File::create("sitemap.xml").unwrap();
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
            println!("File disallow.cfg was not found, creating one instead.");
            let chk = File::create("disallow.cfg");
            match chk {
                Ok(_) => {
                    println!("===");
                    println!("Created file disallow.cfg.");
                    println!("This file is used to contain all file extensions which should be excluded from sitemap.");
                    println!("For example, if .pdf is on the list then link 'https://foo.com/bar.pdf' won't be included in the sitemap.");
                    println!("You can also write comments in site.cfg starting the lines with #.");
                }
                Err(_) => {
                    println!("Unable to create file disallow.cfg.")
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
            println!("File change_prio.cfg not found, creating one instead.");
            let chk = File::create("change_prio.cfg");
            match chk {
                Ok(_) => {
                    println!("===");
                    println!("Created file change_prio.cfg.");
                    println!("This file is used to show which queries in url should be lowered in priority.");
                    println!("For each query the first line should contain its name and the second one should contain the number");
                    println!(
                        "For example 0.5 for raise priority for 0.5 or -0.3 to lower it by 0.3."
                    );
                    println!("You can also write comments in site.cfg starting the lines with #.");
                }
                Err(_) => {
                    println!("Unable to create file change_prio.cfg.")
                }
            }
        }
    }
    let mut url = String::new();
    let mut file_writer = BufWriter::new(&file);
    let site = File::open("site.cfg");
    match site {
        Ok(site) => {
            let site = BufReader::new(site);
            for line in site.lines() {
                match line {
                    Ok(line) => {
                        if !line.starts_with("#") {
                            url = line
                        }
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
        }
        Err(_) => {
            println!("File site.cfg not found, creating one instead.");
            let chk = File::create("site.cfg");
            match chk {
                Ok(_) => {
                    println!("===");
                    println!("Created file site.cfg.");
                    println!("Now you can use site.cfg to store URL that is going to be mapped.");
                    println!("You can also write comments in site.cfg starting the lines with #.");
                    println!("\nFill in this file and launch the mapper again.");
                    return;
                }
                Err(_) => {
                    println!("Unable to create file site.cfg.")
                }
            }
        }
    }
    let url = Url::parse(&url);
    match url {
        Ok(_) => {
            let mut map = HashMap::new();
            scan_link(url.unwrap(), &mut map, exts, chng);
            println!("\n=====\nTotal: {}\n=====", map.len());
            for (key, value) in map {
                writeln!(
                    &mut file_writer,
                    "{} {} {:.1}",
                    String::from(key.as_str()),
                    " | priority: ",
                    value
                );
            }
        }
        Err(_) => {
            return;
        }
    }
}
