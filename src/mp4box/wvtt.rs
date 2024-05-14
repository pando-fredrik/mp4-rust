use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WvttBox {
    pub data_reference_index: u16,
    pub config: VttCBox,
    pub label: Option<VlabBox>,
    // pub bitrate: Option<BtrtBox>,
}

impl Default for WvttBox {
    fn default() -> Self {
        WvttBox {
            data_reference_index: 1,
            config: VttCBox::default(),
            label: None,
        }
    }
}

impl WvttBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::WvttBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE + 6 + 2;
        size += self.config.box_size();
        if let Some(ref label) = self.label {
            size += label.box_size();
        }
        size
    }
}

impl Mp4Box for WvttBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("data_reference_index={}", self.data_reference_index);
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for WvttBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        skip_bytes(reader, 6)?; // reserved
        let data_reference_index = reader.read_u16::<BigEndian>()?;

        let mut vttc = None;
        let mut vlab = None;

        let mut current = reader.stream_position()?;
        let end = start + size;
        while current < end {
            // Get box header.
            let header = BoxHeader::read(reader)?;
            let BoxHeader { name, size: s } = header;
            if s > size {
                return Err(Error::InvalidData(
                    "wvtt box contains a box with a larger size than it",
                ));
            }

            match name {
                BoxType::VttCBox => {
                    vttc = Some(VttCBox::read_box(reader, s)?);
                }
                BoxType::VlabBox => {
                    vlab = Some(VlabBox::read_box(reader, s)?);
                }
                _ => {
                    // XXX warn!()
                    skip_box(reader, s)?;
                }
            }

            current = reader.stream_position()?;
        }

        if vttc.is_none() {
            return Err(Error::BoxNotFound(BoxType::VttCBox));
        }

        skip_bytes_to(reader, start + size)?;

        Ok(WvttBox {
            data_reference_index,
            config: vttc.unwrap(),
            label: vlab,
        })
    }
}

impl<W: Write> WriteBox<&mut W> for WvttBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        writer.write_u32::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(self.data_reference_index)?;

        self.config.write_box(writer)?;

        if let Some(ref label) = self.label {
            label.write_box(writer)?;
        }

        Ok(size)
    }
}
