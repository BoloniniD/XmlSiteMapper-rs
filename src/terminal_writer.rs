use console::Term;

#[derive(Clone)]
pub struct TermWriter {
    term: Term,
    active: bool,
}

impl TermWriter {
    pub fn new(active: bool) -> TermWriter {
        TermWriter {
            term: Term::stdout(),
            active,
        }
    }

    pub fn print_progress(&self, links: i64, total: usize) {
        if !self.active {
            return;
        }
        self.term.clear_last_lines(2);
        self.term
            .write_line(&format!("Size of queue on this iteration: {}", links));
        self.term
            .write_line(&format!("Total links found: {}", total));
    }

    pub fn start_progress(&self, links: i64, total: usize) {
        if !self.active {
            return;
        }
        self.term
            .write_line(&format!("Size of queue on this iteration: {}", links));
        self.term
            .write_line(&format!("Total links found: {}", total));
    }

    pub fn print_to_term(&self, st: String) {
        if !self.active {
            return;
        }
        self.term.write_line(&st);
    }
}
