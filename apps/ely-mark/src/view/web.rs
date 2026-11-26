//! Web pictures, fetched only when asked or allowed (#120).

use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    time::Duration,
};

use ely_gpui_component::documents::{HeldPicture, Pictures, markdown_tree, mdast::Node};
use ely_mail_engine::fetch;
use gpui::{AppContext as _, Context, ImageFormat, Task};

use super::MarkView;

/// Web pictures come down this many at once.
const FETCHERS: usize = 4;
/// Typing settles this long before a scan.
const SETTLE: Duration = Duration::from_millis(400);

/// Each address's type and bytes, or why not.
type Fetched = Vec<(String, Result<(String, Vec<u8>), String>)>;

#[derive(Default)]
pub(crate) struct Web {
    /// Fetched pictures by address, and why not.
    held: Rc<Pictures>,
    /// Addresses on their way down.
    asked: HashSet<String>,
    /// Loads as the text changes (the documents setting).
    auto: bool,
    /// The scan waiting for typing to settle.
    scan: Option<Task<()>>,
}

impl MarkView {
    /// Loads as text changes, or only when asked.
    pub fn set_web_auto(&mut self, auto: bool, cx: &mut Context<Self>) {
        if self.web.auto == auto {
            return;
        }
        self.web.auto = auto;
        match auto {
            true => self.load_web(false, cx),
            false => self.web.scan = None,
        }
    }

    /// The text changed; a kept setting loads what is new.
    pub(super) fn web_changed(&mut self, cx: &mut Context<Self>) {
        if !self.web.auto {
            return;
        }
        let text = self.document.read(cx).text(cx);
        self.web.scan = Some(cx.spawn(async move |view, cx| {
            cx.background_executor().timer(SETTLE).await;
            let urls = cx.background_spawn(async move { addresses(&text) }).await;
            view.update(cx, |view, cx| view.fetch_web(urls, false, cx)).ok();
        }));
    }

    /// Fetches the text's web pictures; `again` retries failures.
    pub fn load_web(&mut self, again: bool, cx: &mut Context<Self>) {
        let text = self.document.read(cx).text(cx);
        self.web.scan = Some(cx.spawn(async move |view, cx| {
            let urls = cx.background_spawn(async move { addresses(&text) }).await;
            view.update(cx, |view, cx| view.fetch_web(urls, again, cx)).ok();
        }));
    }

    #[cfg(test)]
    pub(crate) fn web_auto(&self) -> bool {
        self.web.auto
    }

    /// The picture at `url` is fetched and drawn.
    pub(super) fn web_held(&self, url: &str) -> bool {
        matches!(self.web.held.get(url), Some(Ok(_)))
    }

    fn fetch_web(&mut self, urls: Vec<String>, again: bool, cx: &mut Context<Self>) {
        let web = &mut self.web;
        let wanted: Vec<String> = urls
            .into_iter()
            .filter(|url| match web.held.get(url) {
                Some(Ok(_)) => false,
                Some(Err(_)) => again,
                None => true,
            })
            .filter(|url| web.asked.insert(url.clone()))
            .collect();
        if wanted.is_empty() {
            return;
        }
        log::info!("mark: fetching {} web pictures", wanted.len());
        let chunk = wanted.len().div_ceil(FETCHERS);
        let fetchers: Vec<_> = (wanted.chunks(chunk).map(<[String]>::to_vec))
            .map(|urls| cx.background_spawn(async move { fetch_each(&urls) }))
            .collect();
        cx.spawn(async move |view, cx| {
            let fetched = futures::future::join_all(fetchers).await.into_iter().flatten().collect();
            view.update(cx, |view, cx| view.web_fetched(fetched, cx)).ok();
        })
        .detach();
    }

    fn web_fetched(&mut self, fetched: Fetched, cx: &mut Context<Self>) {
        let mut held = (*self.web.held).clone();
        let mut failed = 0;
        for (url, got) in fetched {
            self.web.asked.remove(&url);
            let picture = got.and_then(|(kind, bytes)| picture(&kind, bytes));
            let picture = picture.map_err(|error| {
                failed += 1;
                ely_ui::i18n::text(cx, "mark.web_pictures.failed", &[("error", &error)])
            });
            held.insert(url, picture);
        }
        log::info!("mark: web pictures in, {failed} did not load");
        self.web.held = Rc::new(held);
        let held = self.web.held.clone();
        self.document
            .update(cx, |document, cx| document.set_pictures(held, cx));
    }
}

/// The text's web picture addresses, each once.
fn addresses(text: &str) -> Vec<String> {
    let tree = markdown_tree(text);
    let (mut found, mut named, mut urls) = (Vec::new(), Vec::new(), HashMap::new());
    walk(&tree, &mut |node| match node {
        Node::Image(image) => found.push(image.url.clone()),
        Node::ImageReference(image) => named.push(image.identifier.clone()),
        // The first definition wins, as the drawing reads it.
        Node::Definition(definition) => {
            urls.entry(definition.identifier.clone())
                .or_insert_with(|| definition.url.clone());
        }
        _ => {}
    });
    found.extend(named.iter().filter_map(|name| urls.get(name).cloned()));
    let mut seen = HashSet::new();
    found.retain(|url| is_web(url) && seen.insert(url.clone()));
    found
}

fn walk(node: &Node, each: &mut impl FnMut(&Node)) {
    each(node);
    for child in node.children().into_iter().flatten() {
        walk(child, each);
    }
}

pub(super) fn is_web(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Blocking fetches in turn; never logs addresses.
fn fetch_each(urls: &[String]) -> Fetched {
    let fetched = urls.iter().map(|url| (url.clone(), fetch::picture(url).map_err(|err| err.to_string())));
    fetched.collect()
}

/// Fetched bytes as the document draws them.
fn picture(kind: &str, bytes: Vec<u8>) -> Result<HeldPicture, String> {
    let kind = match kind {
        "image/jpg" | "image/pjpeg" => "image/jpeg",
        other => other,
    };
    let format = ImageFormat::from_mime_type(kind).ok_or_else(|| format!("{kind:?} is not drawn"))?;
    // SVG may load files; a web one is not drawn.
    if format == ImageFormat::Svg {
        return Err("SVG from the web is not drawn".into());
    }
    if bytes.is_empty() {
        return Err("the picture is empty".into());
    }
    Ok(HeldPicture::new(format, bytes))
}

#[cfg(test)]
mod tests;
