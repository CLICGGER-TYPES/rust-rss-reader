use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::escape;
use quick_xml::reader::Reader;
use quick_xml::Writer;

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub(crate) struct OpmlOutline {
    pub text: Option<String>,
    pub xml_url: Option<String>,
    #[allow(dead_code)]
    pub html_url: Option<String>,
    pub children: Vec<OpmlOutline>,
}

/// 一个订阅源条目：`（标题, xmlUrl, htmlUrl）`。
pub(crate) type FeedEntry = (String, String, Option<String>);

/// 一个分组：`（分组名, 该分组下的订阅源列表）`。
pub(crate) type FeedGroup = (String, Vec<FeedEntry>);

/// 解析 OPML 文本为 outline 树。
pub(crate) fn parse_opml(content: &str) -> Result<Vec<OpmlOutline>> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut stack: Vec<OpmlOutline> = vec![OpmlOutline {
        text: None,
        xml_url: None,
        html_url: None,
        children: Vec::new(),
    }];

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                if e.name().as_ref() == b"outline" {
                    let node = outline_from_start(&e, reader.decoder())?;
                    stack.push(node);
                }
            }
            Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"outline" {
                    let node = outline_from_start(&e, reader.decoder())?;
                    let len = stack.len();
                    stack[len - 1].children.push(node);
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"outline" {
                    if let Some(node) = stack.pop() {
                        let len = stack.len();
                        stack[len - 1].children.push(node);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::Opml(format!("parse failed: {e}"))),
            _ => {}
        }
        buf.clear();
    }

    let root = stack
        .pop()
        .unwrap_or(OpmlOutline {
            text: None,
            xml_url: None,
            html_url: None,
            children: Vec::new(),
        });
    Ok(root.children)
}

#[allow(deprecated)]
fn outline_from_start(e: &BytesStart, decoder: quick_xml::encoding::Decoder) -> Result<OpmlOutline> {
    let mut attrs = Vec::new();
    for attr in e.attributes() {
        let attr = attr.map_err(|err| Error::Opml(format!("attr: {err}")))?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let value = attr
            .decode_and_unescape_value(decoder)
            .map_err(|err| Error::Opml(format!("attr value: {err}")))?
            .to_string();
        attrs.push((key, value));
    }
    let get = |k: &str| {
        attrs
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.clone())
            .filter(|v| !v.is_empty())
    };
    Ok(OpmlOutline {
        text: get("text").or_else(|| get("title")),
        xml_url: get("xmlUrl"),
        html_url: get("htmlUrl"),
        children: Vec::new(),
    })
}

/// 序列化 outline 树为 OPML 2.0 文本。
pub(crate) fn export_opml(
    title: &str,
    folders: &[FeedGroup],
    ungrouped: &[FeedEntry],
) -> Result<String> {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(opml_err)?;

    let mut opml = BytesStart::new("opml");
    opml.push_attribute(("version", "2.0"));
    writer.write_event(Event::Start(opml)).map_err(opml_err)?;

    writer
        .write_event(Event::Start(BytesStart::new("head")))
        .map_err(opml_err)?;
    writer
        .write_event(Event::Start(BytesStart::new("title")))
        .map_err(opml_err)?;
    writer
        .write_event(Event::Text(quick_xml::events::BytesText::from_escaped(escape::escape(title))))
        .map_err(opml_err)?;
    writer
        .write_event(Event::End(BytesEnd::new("title")))
        .map_err(opml_err)?;
    writer
        .write_event(Event::End(BytesEnd::new("head")))
        .map_err(opml_err)?;

    writer
        .write_event(Event::Start(BytesStart::new("body")))
        .map_err(opml_err)?;

    for (folder, feeds) in folders {
        let mut folder_el = BytesStart::new("outline");
        folder_el.push_attribute(("text", escape::escape(folder).as_ref()));
        writer
            .write_event(Event::Start(folder_el))
            .map_err(opml_err)?;
        for (text, xml_url, html_url) in feeds {
            writer
                .write_event(Event::Empty(outline_el(text, xml_url, html_url)))
                .map_err(opml_err)?;
        }
        writer
            .write_event(Event::End(BytesEnd::new("outline")))
            .map_err(opml_err)?;
    }
    for (text, xml_url, html_url) in ungrouped {
        writer
            .write_event(Event::Empty(outline_el(text, xml_url, html_url)))
            .map_err(opml_err)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("body")))
        .map_err(opml_err)?;
    writer
        .write_event(Event::End(BytesEnd::new("opml")))
        .map_err(opml_err)?;

    String::from_utf8(writer.into_inner()).map_err(|e| Error::Opml(e.to_string()))
}

fn outline_el<'a>(text: &'a str, xml_url: &'a str, html_url: &'a Option<String>) -> BytesStart<'a> {
    let mut el = BytesStart::new("outline");
    el.push_attribute(("type", "rss"));
    el.push_attribute(("text", escape::escape(text).as_ref()));
    el.push_attribute(("title", escape::escape(text).as_ref()));
    el.push_attribute(("xmlUrl", escape::escape(xml_url).as_ref()));
    if let Some(h) = html_url {
        el.push_attribute(("htmlUrl", escape::escape(h).as_ref()));
    }
    el
}

fn opml_err<E: std::fmt::Display>(e: E) -> Error {
    Error::Opml(e.to_string())
}
