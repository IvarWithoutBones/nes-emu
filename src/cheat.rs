//! Cheat code parsing for the FCEUX format.

use std::{fmt, sync::mpsc::Receiver};

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum CheatType {
    ReadSubstitute,
    Compare,
    ReadSubstituteCompare,
}

impl CheatType {
    fn try_from(str: &str) -> Option<Self> {
        match str {
            "S" => Some(Self::ReadSubstitute),
            "C" => Some(Self::Compare),
            "SC" => Some(Self::ReadSubstituteCompare),
            "CS" => Some(Self::ReadSubstituteCompare),
            _ => None,
        }
    }

    pub fn is_compare(&self) -> bool {
        matches!(self, Self::Compare | Self::ReadSubstituteCompare)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Cheat {
    pub ty: CheatType,
    pub address: u16,
    pub value: u8,
    pub description: Option<String>,
}

impl Cheat {
    pub fn from(line: &str) -> Option<Self> {
        let items = line.split(':');

        let mut ty = None;
        let mut addr = None;
        let mut value = None;
        let mut description = None;

        for (index, item) in items.enumerate() {
            match index {
                0 => ty = Some(CheatType::try_from(item)?),
                1 => addr = Some(u16::from_str_radix(item, 16).ok()?),
                2 => value = Some(u8::from_str_radix(item, 16).ok()?),
                3 => {
                    description = Some(item.to_string());
                    break;
                }
                _ => return None,
            }
        }

        Some(Self {
            ty: ty?,
            address: addr?,
            value: value?,
            description,
        })
    }

    pub fn from_multiple(lines: &str) -> impl Iterator<Item = Self> + '_ {
        lines.trim().lines().flat_map(Self::from)
    }
}

impl fmt::Debug for Cheat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cheat")
            .field("type", &self.ty)
            .field("address", &format_args!("{:#04x}", self.address))
            .field("value", &format_args!("{:#02x}", self.value))
            .field("description", &self.description)
            .finish()
    }
}

pub enum CheatRequest {
    Add(Cheat),
    Remove(Cheat),
    Clear,
}

pub struct CheatReceiver {
    pub receiver: Receiver<CheatRequest>,
    pub cheats: Vec<Cheat>,
}

impl CheatReceiver {
    pub fn new(receiver: Receiver<CheatRequest>) -> Self {
        Self {
            receiver,
            cheats: Vec::new(),
        }
    }

    pub fn update(&mut self) {
        while let Ok(request) = self.receiver.try_recv() {
            match request {
                CheatRequest::Add(cheat) => self.cheats.push(cheat),
                CheatRequest::Remove(cheat) => {
                    self.cheats.retain(|c| c != &cheat);
                }
                CheatRequest::Clear => self.cheats.clear(),
            }
        }
    }

    pub fn contains(&self, address: u16) -> Option<&Cheat> {
        self.cheats.iter().find(|c| c.address == address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let cheat = Cheat::from("S:0000:00:Infinite lives").unwrap();
        assert_eq!(
            cheat,
            Cheat {
                ty: CheatType::ReadSubstitute,
                address: 0x0000,
                value: 0x00,
                description: Some("Infinite lives".to_string()),
            }
        );
    }
}
