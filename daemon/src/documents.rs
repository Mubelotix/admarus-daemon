use scraper::{Selector, Html, ElementRef};
use crate::prelude::*;

pub enum Document {
    Html(HtmlDocument),
}

impl Document {
    pub fn words(&self) -> Vec<String> {
        match self {
            Document::Html(html) => html.words(),
        }
    }

    pub fn language(&self) -> Option<String> {
        match self {
            Document::Html(html) => Some(html.language()),
        }
    }

    pub fn into_result(self, cid: String, metadata: Metadata, query: &[String]) -> Option<DocumentResult> {
        match self {
            Document::Html(html) => html.into_result(cid, metadata, query),
        }
    }
}

pub struct HtmlDocument {
    raw: String,
    parsed: scraper::Html,
}

impl HtmlDocument {
    pub fn init(raw: String) -> HtmlDocument {
        let parsed = Html::parse_document(&raw);
        HtmlDocument {
            raw,
            parsed,
        }
    }

    pub fn words(&self) -> Vec<String> {
        let document = &self.parsed;
        let body_selector = Selector::parse("body").unwrap();
        let body_el = document.select(&body_selector).next();
        let body = body_el.map(|el| el.text().collect::<Vec<_>>().join(" ")).unwrap_or_default();
        body.to_lowercase().split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| w.len() >= 3).map(|w| w.to_string()).collect()
    }

    pub fn language(&self) -> String {
        let document = &self.parsed;
        let html_selector = Selector::parse("html").unwrap();
        let html_el = document.select(&html_selector).next();
        let Some(lang) = html_el.and_then(|el| el.value().attr("lang").map(|lang| lang.trim().to_string())) else {
            return String::from("unknown");
        };
        let mut lang = lang.split('-');
        
        lang.next().unwrap_or("unknown").to_string()
    }

    #[allow(clippy::question_mark)]
    pub fn into_result(self, cid: String, metadata: Metadata, query: &[String]) -> Option<DocumentResult> {
        let document = &self.parsed;

        // Retrieve title
        let title_selector = Selector::parse("title").unwrap();
        let title_el = document.select(&title_selector).next();
        let mut title = title_el.map(|el| el.text().collect::<Vec<_>>().join(" "));
        if title.as_ref().map(|t| t.trim().is_empty()).unwrap_or(false) {
            title = None;
        }

        // Retrieve h1
        let mut h1 = None;
        if title.is_none() {
            let h1_selector = Selector::parse("h1").unwrap();
            let h1_el = document.select(&h1_selector).next();
            h1 = h1_el.map(|el| el.text().collect::<Vec<_>>().join(" "));
            if h1.as_ref().map(|t| t.trim().is_empty()).unwrap_or(false) {
                h1 = None;
            }
        }
        
        if title.is_none() && h1.is_none() {
            return None;
        }

        // Retrieve description
        let description_selector = Selector::parse("meta[name=description]").unwrap();
        let description_el = document.select(&description_selector).next();
        let description = description_el.map(|el| el.value().attr("content").unwrap().to_string());

        // Retrieve the most relevant extract
        fn extract_score(extract: &str, query: &[String]) -> usize {
            let mut score = 0;
            let mut extract_words = extract.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| w.len() >= 3).map(|w| w.to_lowercase()).collect::<Vec<_>>();
            if extract_words.is_empty() {
                return 0;
            }
            let first_word = extract_words.remove(0);
            if query.contains(&first_word) {
                score += 4;
            }
            for word in query {
                if extract_words.contains(word) {
                    score += 1;
                }
            }
            score
        }
        let body = document.select(&Selector::parse("body").unwrap()).next().unwrap();
        let fragments = body.text().collect::<Vec<_>>();
        let mut best_extract = "";
        let mut best_extract_score = 0;
        for fragment in fragments {
            if fragment.len() >= 350 || fragment.len() <= 50 {
                continue;
            }
            let score = extract_score(fragment, query);
            if score > best_extract_score {
                best_extract_score = score;
                best_extract = fragment;
            }
        }
        let extract = match best_extract_score > 0 {
            true => Some(best_extract.to_string()),
            false => None,
        };
        
        if description.is_none() && extract.is_none() {
            return None;
        }

        // Count words
        #[allow(clippy::too_many_arguments)]
        fn count_words(
            el: ElementRef, query: &[String], term_counts: &mut Vec<WordCount>, word_count: &mut WordCount, 
            mut h1: bool, mut h2: bool, mut h3: bool, mut h4: bool, mut h5: bool, mut h6: bool, mut strong: bool, mut em: bool, mut small: bool, mut s: bool
        ) {
            match el.value().name() {
                "h1" => h1 = true,
                "h2" => h2 = true,
                "h3" => h3 = true,
                "h4" => h4 = true,
                "h5" => h5 = true,
                "h6" => h6 = true,
                "strong" => strong = true,
                "em" => em = true,
                "small" => small = true,
                "s" => s = true,
                _ => (),
            }
            for child in el.children() {
                match child.value() {
                    scraper::node::Node::Element(_) => {
                        let child_ref = ElementRef::wrap(child).unwrap();
                        count_words(child_ref, query, term_counts, word_count, h1, h2, h3, h4, h5, h6, strong, em, small, s)
                    },
                    scraper::node::Node::Text(text) => {
                        let text = text.to_lowercase();
                        let words = text
                            .split(|c: char| !c.is_ascii_alphanumeric())
                            .filter(|w| w.len() >= 3)
                            .map(|w| w.to_string());
                        for word in words {
                            if let Some(i) = query.iter().position(|q| q == &word) {
                                let term_count = term_counts.get_mut(i).unwrap();
                                term_count.add(h1, h2, h3, h4, h5, h6, strong, em, small, s)
                            }
                            word_count.add(h1, h2, h3, h4, h5, h6, strong, em, small, s);
                        }
                    },
                    _ => (),
                }
            }
        }
        let mut term_counts = query.iter().map(|_| WordCount::default()).collect::<Vec<_>>();
        let mut word_count = WordCount::default();
        count_words(body, query, &mut term_counts, &mut word_count, false, false, false, false, false, false, false, false, false, false);

        Some(DocumentResult {
            cid,
            paths: metadata.paths,
            icon_cid: None,
            domain: None,
            title,
            h1,
            description,
            extract,

            term_counts,
            word_count,
        })
    }
}
