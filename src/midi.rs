use std::fs::File;
use std::io::{BufReader, Read};
use std::ops::{AddAssign, ShlAssign};
use std::path::PathBuf;

use anyhow::Context;

use crate::error::Error;

/// In-memory representation of the contents of a MIDI file.
#[derive(Debug)]
pub struct MidiFile {
    format: Format,
    division: Division,
}

impl MidiFile {
    /// Reads and parses the contents of the given file.
    pub fn read(path: &PathBuf) -> Result<Self, Error> {
        let file = File::open(path).context("failed to open file")?;
        let mut buffer = Vec::new();
        let mut reader = BufReader::new(file);
        reader
            .read_to_end(&mut buffer)
            .context("failed to read file")?;

        let mut chunker = Chunker::new(&buffer);
        let mut chunks = Vec::new();

        while !chunker.done() {
            let r#type: u32 = chunker.claim_as(4)?;
            let length: u32 = chunker.claim_as(4)?;
            let data = chunker.claim(length)?;
            chunks.push(Chunk { r#type, data });
        }

        if chunks.is_empty() {
            return Err(Error::general("an empty file cannot be a valid MIDI file"));
        }

        let mut header_chunker = Chunker::new(chunks[0].data);
        let format = header_chunker.claim_as::<u16>(2)?.try_into()?;

        if header_chunker.claim_as::<usize>(2)?
            != chunks
                .iter()
                .filter(|chunk| {
                    chunk
                        .type_variant()
                        .map(|chunk| chunk == ChunkType::Track)
                        .unwrap_or(false)
                })
                .count()
        {
            return Err(Error::general(
                "ntrks header value does not match the number of track chunks in the file",
            ));
        }

        let division = header_chunker.claim_as::<i16>(2)?.try_into()?;

        Ok(Self { format, division })
    }
}

/// Format of a MIDI file, as specified in the file header chunk.
#[derive(Debug)]
pub enum Format {
    /// File contains a single, multi channel track.
    MultiChannel,
    /// File contains one or more simultaneous tracks (or MIDI outputs) of a sequence.
    Simultaneous,
    /// File contains one or more sequentially independent single-track patterns.
    Independent,
}

impl TryFrom<u16> for Format {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::MultiChannel),
            1 => Ok(Self::Simultaneous),
            2 => Ok(Self::Independent),
            other => Err(Error::general(&format!(
                "invalid integer value for format: {other}"
            ))),
        }
    }
}

/// Specifies the meaning of the delta times in the MIDI file.
#[derive(Debug)]
pub enum Division {
    Metrical {
        ticks_per_quarter_note: u16,
    },
    TimeCode {
        smpte_format: SmpteFormat,
        // TODO: Supposedly this is in 2's complement??
        ticks_per_frame: u8,
    },
}

impl TryFrom<i16> for Division {
    type Error = Error;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        if value >= 0 {
            Ok(Self::Metrical {
                ticks_per_quarter_note: value as u16,
            })
        } else {
            Ok(Self::TimeCode {
                smpte_format: SmpteFormat::try_from(value >> 8)?,
                ticks_per_frame: (value & 0xFF) as u8,
            })
        }
    }
}

/// Standardized FPS rates for MIDI.
#[derive(Debug)]
pub enum SmpteFormat {
    Fps24,
    Fps25,
    Fps30Drop,
    Fps30,
}

impl TryFrom<i16> for SmpteFormat {
    type Error = Error;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            -24 => Ok(Self::Fps24),
            -25 => Ok(Self::Fps25),
            -29 => Ok(Self::Fps30Drop),
            -30 => Ok(Self::Fps30),
            other => Err(Error::general(&format!(
                "invalid integer value for smpte format: {other}"
            ))),
        }
    }
}

/// A raw variable-sized chunk of data from a MIDI file.
#[derive(Debug)]
pub struct Chunk<'a> {
    r#type: u32,
    data: &'a [u8],
}

impl Chunk<'_> {
    /// Returns the type of the chunk in its 4 character string representation.
    pub fn type_str(&self) -> String {
        let mut chars = ['\0'; 4];
        let mut temp = self.r#type;
        for i in 0..4 {
            chars[i] = ((temp & 0xFF) as u8) as char;
            temp >>= 8;
        }
        chars.iter().rev().collect()
    }

    /// Returns the type of the chunk as a valid enum.
    pub fn type_variant(&self) -> Result<ChunkType, Error> {
        self.type_str()
            .parse()
            .context("failed to parse chunk type")
            .map_err(Into::into)
    }
}

/// The different types of chunks in a MIDI file.
#[derive(Debug, Eq, PartialEq, strum::EnumString)]
pub enum ChunkType {
    #[strum(serialize = "MThd")]
    Header,
    #[strum(serialize = "MTrk")]
    Track,
}

/// Helper type for tracking progress in the [`chunk`] function.
struct Chunker<'a> {
    position: usize,
    buffer: &'a [u8],
}

impl<'a> Chunker<'a> {
    /// Creates a new instance of [`Chunker`].
    fn new(buffer: &'a [u8]) -> Self {
        Self {
            position: 0,
            buffer,
        }
    }

    /// Advance the chunker cursor by the given number of bytes and return its eclipsed slice.
    fn claim(&mut self, bytes: u32) -> Result<&'a [u8], Error> {
        if self.done() {
            return Err(Error::general("chunker is done"));
        }

        let next = self.position + bytes as usize;
        let next = next.min(self.buffer.len());
        let slice = &self.buffer[self.position..next];
        self.position = next;

        Ok(slice)
    }

    /// Advance the chunker cursor by the given number of bytes and return its eclipsed data as
    /// a single unsigned integer value.
    fn claim_as<T>(&mut self, bytes: u32) -> Result<T, Error>
    where
        T: AddAssign<T> + From<u8> + ShlAssign<u8>,
    {
        let slice = self.claim(bytes)?;
        let mut value = T::from(0);
        for byte in slice {
            value <<= 8;
            value += T::from(*byte);
        }

        Ok(value)
    }

    /// Returns whether the chunker is done.
    fn done(&self) -> bool {
        self.position >= self.buffer.len()
    }
}
