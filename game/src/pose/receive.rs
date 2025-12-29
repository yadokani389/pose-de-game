use std::io::{self, Read};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageFormat, ImageSampler, ImageType},
    prelude::*,
};
use serde::Deserialize;
use serde_bytes::ByteBuf;

use crate::{SocketResource, args::Args};

use super::{PeopleData, PeopleDataRes, PersonData};

#[derive(Deref, DerefMut)]
pub(super) struct SocketBuffer(Vec<u8>);

impl Default for SocketBuffer {
    fn default() -> Self {
        Self(vec![0; 1_000_000])
    }
}

#[derive(Default)]
pub(super) struct StreamState {
    prefix: [u8; 4],
    prefix_read: usize,
    expected_len: Option<usize>,
    body_read: usize,
}

impl StreamState {
    fn reset(&mut self) {
        self.prefix = [0; 4];
        self.prefix_read = 0;
        self.expected_len = None;
        self.body_read = 0;
    }
}

enum StreamOutcome {
    Incomplete,
    Closed,
    MessageReady(usize),
}

#[derive(Deserialize, Debug)]
struct PersonPayload {
    pub keypoints: Vec<Option<[f64; 2]>>,
    pub right_hand_closed: Option<bool>,
    pub left_hand_closed: Option<bool>,
    #[serde(default)]
    pub person_png: Option<ByteBuf>,
}

impl PersonPayload {
    fn into_person_data(self, idx: usize) -> PersonData {
        let PersonPayload {
            keypoints,
            right_hand_closed,
            left_hand_closed,
            person_png,
        } = self;

        let person_image = match person_png {
            Some(bytes) => match decode_person_png(bytes.as_ref()) {
                Ok(image) => Some(image),
                Err(err) => {
                    error!("Failed to decode PNG for person {idx}: {err}");
                    None
                }
            },
            None => None,
        };

        PersonData {
            keypoints,
            right_hand_closed,
            left_hand_closed,
            person_image,
        }
    }

    fn into_person_data_without_image(self) -> PersonData {
        let PersonPayload {
            keypoints,
            right_hand_closed,
            left_hand_closed,
            person_png: _,
        } = self;

        PersonData {
            keypoints,
            right_hand_closed,
            left_hand_closed,
            person_image: None,
        }
    }
}

type PeoplePayload = Vec<PersonPayload>;

pub fn receive_data(
    mut socket: ResMut<SocketResource>,
    args: Res<Args>,
    mut people_data: ResMut<PeopleDataRes>,
    mut buffer: Local<SocketBuffer>,
    mut stream_state: Local<StreamState>,
) {
    let Ok(Some(size)) = recv_message(&mut socket, &mut buffer, &mut stream_state) else {
        return;
    };

    match serde_cbor::from_slice::<PeoplePayload>(&buffer[..size]) {
        Ok(people) => {
            let converted: PeopleData = if args.show_person {
                people
                    .into_iter()
                    .enumerate()
                    .map(|(idx, person)| person.into_person_data(idx))
                    .collect()
            } else {
                people
                    .into_iter()
                    .map(PersonPayload::into_person_data_without_image)
                    .collect()
            };

            **people_data = converted;
        }
        Err(e) => {
            error!("Failed to parse CBOR data: {e}");
        }
    }
}

fn decode_person_png(bytes: &[u8]) -> Result<Image, TextureError> {
    let image = Image::from_buffer(
        bytes,
        ImageType::Format(ImageFormat::Png),
        CompressedImageFormats::empty(),
        true,
        ImageSampler::Default,
        RenderAssetUsages::default(),
    )?;

    Ok(image)
}

fn recv_message(
    socket: &mut SocketResource,
    buffer: &mut SocketBuffer,
    stream_state: &mut StreamState,
) -> io::Result<Option<usize>> {
    match socket {
        SocketResource::Tcp { listener, stream } => {
            recv_tcp_stream(listener, stream, buffer, stream_state)
        }
        #[cfg(unix)]
        SocketResource::UnixStream { listener, stream } => {
            recv_unix_stream(listener, stream, buffer, stream_state)
        }
    }
}

fn recv_tcp_stream(
    listener: &TcpListener,
    stream_opt: &mut Option<TcpStream>,
    buffer: &mut SocketBuffer,
    state: &mut StreamState,
) -> io::Result<Option<usize>> {
    if stream_opt.is_none() {
        match listener.accept() {
            Ok((stream, _addr)) => {
                stream.set_nonblocking(true)?;
                *stream_opt = Some(stream);
                state.reset();
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(err) => return Err(err),
        }
    }

    let stream = match stream_opt.as_mut() {
        Some(s) => s,
        None => return Ok(None),
    };

    match read_length_prefixed(stream, buffer, state)? {
        StreamOutcome::Incomplete => Ok(None),
        StreamOutcome::Closed => drop_stream_tcp(stream_opt, state),
        StreamOutcome::MessageReady(size) => Ok(Some(size)),
    }
}

#[cfg(unix)]
fn recv_unix_stream(
    listener: &UnixListener,
    stream_opt: &mut Option<UnixStream>,
    buffer: &mut SocketBuffer,
    state: &mut StreamState,
) -> io::Result<Option<usize>> {
    if stream_opt.is_none() {
        match listener.accept() {
            Ok((stream, _addr)) => {
                stream.set_nonblocking(true)?;
                *stream_opt = Some(stream);
                state.reset();
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(err) => return Err(err),
        }
    }

    let stream = match stream_opt.as_mut() {
        Some(s) => s,
        None => return Ok(None),
    };

    match read_length_prefixed(stream, buffer, state)? {
        StreamOutcome::Incomplete => Ok(None),
        StreamOutcome::Closed => drop_stream_unix(stream_opt, state),
        StreamOutcome::MessageReady(size) => Ok(Some(size)),
    }
}

#[cfg(unix)]
fn drop_stream_unix(
    stream_opt: &mut Option<UnixStream>,
    state: &mut StreamState,
) -> io::Result<Option<usize>> {
    *stream_opt = None;
    state.reset();
    Ok(None)
}

fn drop_stream_tcp(
    stream_opt: &mut Option<TcpStream>,
    state: &mut StreamState,
) -> io::Result<Option<usize>> {
    *stream_opt = None;
    state.reset();
    Ok(None)
}

fn read_length_prefixed<R: Read>(
    stream: &mut R,
    buffer: &mut SocketBuffer,
    state: &mut StreamState,
) -> io::Result<StreamOutcome> {
    // Read length prefix
    if state.expected_len.is_none() {
        while state.prefix_read < 4 {
            match stream.read(&mut state.prefix[state.prefix_read..4]) {
                Ok(0) => return Ok(StreamOutcome::Closed),
                Ok(n) => state.prefix_read += n,
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    return Ok(StreamOutcome::Incomplete);
                }
                Err(err) => return Err(err),
            }
        }
        let len = u32::from_be_bytes(state.prefix) as usize;
        state.expected_len = Some(len);
        if buffer.len() < len {
            buffer.resize(len, 0);
        }
    }

    let expected = state.expected_len.unwrap();
    while state.body_read < expected {
        match stream.read(&mut buffer[state.body_read..expected]) {
            Ok(0) => return Ok(StreamOutcome::Closed),
            Ok(n) => state.body_read += n,
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                return Ok(StreamOutcome::Incomplete);
            }
            Err(err) => return Err(err),
        }
    }

    let size = expected;
    state.reset();
    Ok(StreamOutcome::MessageReady(size))
}
