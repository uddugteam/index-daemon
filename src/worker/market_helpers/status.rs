use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub enum Status {
    Active,
    Down,
}
impl Status {
    pub fn is_active(&self) -> bool {
        matches!(self, Status::Active)
    }

    pub fn is_down(&self) -> bool {
        matches!(self, Status::Down)
    }
}
// TODO: Replace error type with correct
impl FromStr for Status {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "down" => Ok(Self::Down),
            _ => Err(()),
        }
    }
}
impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let status_string = match self {
            Status::Active => "active",
            Status::Down => "down",
        };

        write!(f, "{}", status_string)
    }
}
impl Clone for Status {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for Status {}
