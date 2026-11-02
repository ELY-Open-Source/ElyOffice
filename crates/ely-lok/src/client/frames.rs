//! The worker's output: replies and pushed events (LOK-03).

use std::{io, sync::mpsc};

use super::Feed;
use crate::events::{A11y, Events, Rect};
use crate::ipc::{self, Unpack, tag};

/// One pushed event batch: its document and events.
fn unpack_pushed(body: &[u8]) -> io::Result<(u64, Events)> {
    let mut unpack = Unpack::new(body);
    let (id, revision) = (unpack.u64()?, unpack.u64()?);
    let (seq, generation, all) = (unpack.u64()?, unpack.u64()?, unpack.u64()? != 0);
    let rects = rects_of(&mut unpack)?;
    let cursor = rects_of(&mut unpack)?.first().copied();
    let cursor_visible = unpack.u64()? != 0;
    let pointer = unpack.str()?;
    let selection = rects_of(&mut unpack)?;
    let graphic = rects_of(&mut unpack)?.first().copied();
    let cropping = unpack.u64()? != 0;
    let review = unpack.u64()?;
    let searched = unpack.u64()?;
    let resized = unpack.u64()?;
    let dismissed = unpack.u64()?;
    let found = rects_of(&mut unpack)?;
    let menu = Some(unpack.str()?).filter(|menu| !menu.is_empty());
    let faults = (0..unpack.u64()?)
        .map(|_| unpack.str())
        .collect::<io::Result<_>>()?;
    let fonts = (0..unpack.u64()?)
        .map(|_| unpack.str())
        .collect::<io::Result<_>>()?;
    let states = (0..unpack.u64()?)
        .map(|_| Ok((unpack.str()?, unpack.str()?)))
        .collect::<io::Result<_>>()?;
    let blocked = (0..unpack.u64()?)
        .map(|_| Ok((unpack.str()?, unpack.str()?)))
        .collect::<io::Result<_>>()?;
    let links = (0..unpack.u64()?)
        .map(|_| unpack.str())
        .collect::<io::Result<_>>()?;
    let tabs = (0..unpack.u64()?)
        .map(|_| Ok(unpack.i64()? as i32))
        .collect::<io::Result<Vec<_>>>()?;
    let text = unpack.str()?;
    let mut at = || -> io::Result<Option<usize>> { Ok(usize::try_from(unpack.i64()?).ok()) };
    let caret = at()?;
    let marked = at()?.zip(at()?);
    let a11y = A11y {
        text,
        caret,
        selection: marked,
    };
    let cell = crate::events::Cell::unpack(&mut unpack)?;
    let table = crate::table::Table::unpack(&mut unpack)?;
    let text = crate::events::TextAnchors::unpack(&mut unpack)?;
    let events = Events {
        seq,
        revision,
        generation,
        all,
        rects,
        cursor,
        cursor_visible,
        pointer,
        selection,
        text,
        graphic,
        cropping,
        review,
        searched,
        resized,
        dismissed,
        found,
        menu,
        faults,
        fonts,
        states,
        blocked,
        links,
        a11y,
        tabs,
        cell,
        table,
    };
    Ok((id, events))
}

/// Splits the worker's output: replies to requests, pushed events.
pub(super) fn read_frames(
    mut stdout: impl io::Read,
    feed: &Feed,
    replies: &mpsc::Sender<io::Result<(u8, u64, Vec<u8>)>>,
) {
    loop {
        let frame = match ipc::read_frame(&mut stdout) {
            Ok(Some((tag::EVENTS, _, body))) => match unpack_pushed(&body) {
                Ok((id, batch)) => {
                    feed.push(id, batch);
                    continue;
                }
                Err(error) => Err(error),
            },
            Ok(Some(frame)) => Ok(Some(frame)),
            Ok(None) => Ok(None),
            Err(error) => Err(error),
        };
        // The reason output stopped reaches the waiting request.
        let ended = match frame {
            Ok(Some(frame)) => {
                if replies.send(Ok(frame)).is_err() {
                    return;
                }
                continue;
            }
            Ok(None) => io::Error::new(io::ErrorKind::UnexpectedEof, "the lok worker closed its output"),
            Err(error) => {
                log::error!("lok: the worker output is unreadable: {error}");
                error
            }
        };
        feed.died();
        let _ = replies.send(Err(ended));
        return;
    }
}

/// A counted rect list off the wire.
fn rects_of(unpack: &mut Unpack) -> io::Result<Vec<Rect>> {
    let count = unpack.u64()? as usize;
    let mut rects = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        rects.push((
            unpack.i64()? as i32,
            unpack.i64()? as i32,
            unpack.i64()? as i32,
            unpack.i64()? as i32,
        ));
    }
    Ok(rects)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::read_frames;
    use crate::client::Feed;

    #[test]
    fn a_broken_output_reaches_the_waiting_request() {
        let (sender, replies) = mpsc::channel();
        // A header claiming more than the cap.
        read_frames(&u32::MAX.to_le_bytes()[..], &Feed::default(), &sender);
        let error = replies.recv().expect("a message").expect_err("an error");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData, "{error}");
        let (sender, replies) = mpsc::channel();
        read_frames(&[][..], &Feed::default(), &sender);
        let error = replies.recv().expect("a message").expect_err("an error");
        assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof, "{error}");
    }
}
