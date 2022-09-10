/// Fix type
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FixType {
    Invalid,
    Gps,
    DGps,
    /// Precise Position Service
    Pps,
    Rtk,
    FloatRtk,
    Estimated,
    Manual,
    Simulation,
}

impl FixType {
    #[inline]
    pub fn is_valid(self) -> bool {
        match self {
            FixType::Simulation | FixType::Manual | FixType::Estimated | FixType::Invalid => false,
            FixType::DGps | FixType::Gps | FixType::Rtk | FixType::FloatRtk | FixType::Pps => true,
        }
    }
}

impl From<char> for FixType {
    fn from(x: char) -> Self {
        match x {
            '0' => FixType::Invalid,
            '1' => FixType::Gps,
            '2' => FixType::DGps,
            '3' => FixType::Pps,
            '4' => FixType::Rtk,
            '5' => FixType::FloatRtk,
            '6' => FixType::Estimated,
            '7' => FixType::Manual,
            '8' => FixType::Simulation,
            _ => FixType::Invalid,
        }
    }
}
