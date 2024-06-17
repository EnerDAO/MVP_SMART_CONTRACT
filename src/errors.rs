use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 100,
    OnlyPositiveValue = 101,
    NotStarted = 102,
    NotFinished = 103,
    AlreadyFinished = 104,
    TargetNotReached = 105,
    TargetOverreached = 106,
    NoCollateral = 107,
    NothingToClaim = 108,
    AlreadyClaimed = 109,
    ReturnOverreached = 110,
    NotAllowed = 120,
}
