pub mod flash;
pub mod configuration;

#[derive(Default, Debug, Clone)]
pub enum Screen {
    #[default]
    Home,
    BoardSelection,
    ImageSelection,
    DestinationSelection,
    ExtraConfiguration,
    Flashing(flash::FlashingScreen),
}
