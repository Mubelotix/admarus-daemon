use crate::prelude::*;
use html5ever::{tokenizer::{TokenSink, Token as HtmlToken, TokenSinkResult, TagKind, Tokenizer, TokenizerOpts, BufferQueue}, LocalName};
pub enum Document {
    Html(HtmlDocument),
}

impl Document {
    pub fn into_parts(self) -> Option<(Vec<String>, Option<String>)> {
        match self {
            Document::Html(html) => html.into_parts(),
        }
    }

    pub fn into_result(self, cid: String, metadata: Metadata, query: &Query) -> Option<DocumentResult> {
        match self {
            Document::Html(html) => html.into_result(cid, metadata, query),
        }
    }
}
pub struct HtmlDocument {
    raw: String,
}

impl HtmlDocument {
    pub fn init(raw: String) -> HtmlDocument {
        HtmlDocument {
            raw,
        }
    }

    pub fn into_parts(self) -> Option<(Vec<String>, Option<String>)> {
        /*let document = &self.parsed;
        let body_selector = Selector::parse("body").unwrap();
        let body_el = document.select(&body_selector).next();
        let body = body_el.map(|el| el.text().collect::<Vec<_>>().join(" ")).unwrap_or_default();
        body.to_lowercase().split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| w.len() >= 3).map(|w| w.to_string()).collect()*/
        pub(crate) struct HtmlSink<'a> {
            pub(crate) words: &'a mut Vec<String>,
            pub(crate) panic: &'a mut Option<String>,
            pub(crate) opened_elements: Vec<LocalName>,
        }
        
        impl<'a> TokenSink for HtmlSink<'a> {
            type Handle = ();
        
            fn process_token(&mut self, token: HtmlToken, line_number: u64) -> TokenSinkResult<()> {
                if self.panic.is_some() { return TokenSinkResult::Continue }

                match token {
                    HtmlToken::TagToken(tag) => {
                        if tag.self_closing || ["area", "base", "br", "col", "embed", "hr", "img", "input", "keygen", "link", "meta", "param", "source", "track", "wbr"].contains(&tag.name.as_ref()) {
                            return TokenSinkResult::Continue;
                        }
                        match tag.kind {
                            TagKind::StartTag => self.opened_elements.push(tag.name),
                            TagKind::EndTag => match self.opened_elements.pop() {
                                Some(name) => {
                                    if name != tag.name {
                                        *self.panic = Some(format!("Unexpected closing tag </{}> at line {line_number} (expected {name})", tag.name));
                                        return TokenSinkResult::Continue;
                                    }
                                },
                                None => {
                                    *self.panic = Some(format!("Unexpected closing tag </{}> at line {line_number} (no opening tag)", tag.name));
                                    return TokenSinkResult::Continue;
                                }
                            }
                        }
                    },
                    HtmlToken::CharacterTokens(text) => {
                        let text = text.to_lowercase();
                        let new_words = text.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| w.len() >= 3).map(|w| w.to_string());
                        self.words.extend(new_words);
                    },
                    HtmlToken::NullCharacterToken | HtmlToken::CommentToken(_) | HtmlToken::EOFToken | HtmlToken::DoctypeToken(_) => (),
                    HtmlToken::ParseError(e) => {
                        //*self.panic = Some(format!("Parse error: {e}"));
                        return TokenSinkResult::Continue;
                    },
                }
                TokenSinkResult::Continue
            }
        }

        let mut words = Vec::new();
        let mut panic = None;

        let html_sink = HtmlSink { words: &mut words, opened_elements: Vec::new(), panic: &mut panic };
        let mut html_tokenizer = Tokenizer::new(html_sink, TokenizerOpts::default());
        let mut buffer_queue = BufferQueue::new();
        buffer_queue.push_back(self.raw.into());
        let _  = html_tokenizer.feed(&mut buffer_queue);
        html_tokenizer.end();

        match panic {
            Some(panic) => {
                error!("{}", panic);
                None
            },
            None => Some((words, None)),
        }
    }

    #[allow(clippy::question_mark)]
    pub fn into_result(self, cid: String, metadata: Metadata, query: &Query) -> Option<DocumentResult> {
        /*let document = &self.parsed;

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
        fn extract_score(extract: &str, query_positive_terms: &[&String]) -> usize {
            let mut score = 0;
            let mut extract_words = extract.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| w.len() >= 3).map(|w| w.to_lowercase()).collect::<Vec<_>>();
            if extract_words.is_empty() {
                return 0;
            }
            let first_word = extract_words.remove(0);
            if query_positive_terms.contains(&&first_word) {
                score += 4;
            }
            for query_positive_term in query_positive_terms {
                if extract_words.contains(query_positive_term) {
                    score += 1;
                }
            }
            score
        }
        let body = document.select(&Selector::parse("body").unwrap()).next().unwrap();
        let query_positive_terms = query.positive_terms();
        let fragments = body.text().collect::<Vec<_>>();
        let mut best_extract = "";
        let mut best_extract_score = 0;
        for fragment in fragments {
            if fragment.len() >= 350 || fragment.len() <= 50 {
                continue;
            }
            let score = extract_score(fragment, &query_positive_terms);
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
            el: ElementRef, query_positive_terms: &[&String], term_counts: &mut Vec<WordCount>, word_count: &mut WordCount, 
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
                        count_words(child_ref, query_positive_terms, term_counts, word_count, h1, h2, h3, h4, h5, h6, strong, em, small, s)
                    },
                    scraper::node::Node::Text(text) => {
                        let text = text.to_lowercase();
                        let words = text
                            .split(|c: char| !c.is_ascii_alphanumeric())
                            .filter(|w| w.len() >= 3)
                            .map(|w| w.to_string());
                        for word in words {
                            if let Some(i) = query_positive_terms.iter().position(|q| *q == &word) {
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
        let mut term_counts = query_positive_terms.iter().map(|_| WordCount::default()).collect::<Vec<_>>();
        let mut word_count = WordCount::default();
        count_words(body, &query_positive_terms, &mut term_counts, &mut word_count, false, false, false, false, false, false, false, false, false, false);

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
        })*/
        todo!()
    }
}
