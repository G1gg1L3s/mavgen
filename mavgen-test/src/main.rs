#![cfg(feature = "mavgen-test")]

use clap::{Parser, ValueEnum};
use mavlink_core::{error::MessageReadError, MavHeader, MavlinkVersion};

#[derive(Debug, Clone, ValueEnum)]
#[clap(rename_all = "lower")]
enum Dialect {
    #[cfg(feature = "all")]
    All,
    #[cfg(feature = "ardupilotmega")]
    Ardupilotmega,
    #[cfg(feature = "asluav")]
    Asluav,
    #[cfg(feature = "avssuas")]
    Avssuas,
    #[cfg(feature = "common")]
    Common,
    #[cfg(feature = "cubepilot")]
    Cubepilot,
    #[cfg(feature = "development")]
    Development,
    #[cfg(feature = "matrixpilot")]
    Matrixpilot,
    #[cfg(feature = "paparazzi")]
    Paparazzi,
    #[cfg(feature = "storm32")]
    Storm32,
    #[cfg(feature = "u_avionix")]
    UAvionix,
    #[cfg(feature = "ualberta")]
    Ualberta,
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    dialect: Dialect,

    #[arg(long)]
    address: String,
}

#[cfg(not(feature = "serde"))]
trait Message: mavlink_core::Message {}

#[cfg(feature = "serde")]
trait Message: mavlink_core::Message + serde::Serialize + for<'a> serde::Deserialize<'a> {}

#[cfg(feature = "serde")]
fn test_serde<M: Message>(message: M) -> M {
    let json = serde_json::to_string(&message).unwrap();
    serde_json::from_str(&json).unwrap()
}

#[allow(unused)]
fn test<M: Message>(address: &str, heartbeat: M, is_hearbeat: impl FnOnce(&M) -> bool) {
    let connection = mavlink_core::connect::<M>(address).unwrap();

    let heartbeat = mavlink_core::MavFrame {
        header: MavHeader::default(),
        protocol_version: MavlinkVersion::V2,
        msg: heartbeat,
    };

    connection.send_frame(&heartbeat).unwrap();
    let (_, data) = connection.recv().unwrap();

    if !is_hearbeat(&data) {
        panic!("received no heartbeat")
    }

    loop {
        let (_, data) = match connection.recv() {
            Ok(ok) => ok,
            Err(MessageReadError::Io(io)) if io.kind() == std::io::ErrorKind::UnexpectedEof => {
                break
            }
            Err(err) => panic!("{:?}", err),
        };
        println!("RECEIVED {:?}", data.message_name());

        #[cfg(feature = "serde")]
        let data = test_serde(data);

        connection.send_default(&data).unwrap();
        println!("SENT {:?}", data.message_name());
    }
}

#[allow(unused)]
macro_rules! test_dialect {
    ($dialect:ident, $address:expr) => {{
        impl Message for mavgen_test::messages::$dialect::MavMessage {}

        use mavgen_test::messages::$dialect::{
            Heartbeat, MavAutopilot, MavMessage, MavModeFlag, MavState, MavType,
        };
        test(
            $address,
            MavMessage::Heartbeat(Heartbeat {
                custom_mode: 0,
                r#type: MavType::MavTypeOnboardController,
                autopilot: MavAutopilot::MavAutopilotInvalid,
                base_mode: MavModeFlag::empty(),
                system_status: MavState::MavStateActive,
                mavlink_version: 2,
            }),
            |mav_message| matches!(mav_message, MavMessage::Heartbeat(..)),
        )
    }};
}

fn main() {
    #[allow(unused)]
    let args = Args::parse();

    #[allow(unreachable_code)]
    match args.dialect {
        #[cfg(feature = "all")]
        Dialect::All => test_dialect!(all, &args.address),
        #[cfg(feature = "ardupilotmega")]
        Dialect::Ardupilotmega => test_dialect!(ardupilotmega, &args.address),
        #[cfg(feature = "asluav")]
        Dialect::Asluav => test_dialect!(asluav, &args.address),
        #[cfg(feature = "avssuas")]
        Dialect::Avssuas => test_dialect!(avssuas, &args.address),
        #[cfg(feature = "common")]
        Dialect::Common => test_dialect!(common, &args.address),
        #[cfg(feature = "cubepilot")]
        Dialect::Cubepilot => test_dialect!(cubepilot, &args.address),
        #[cfg(feature = "development")]
        Dialect::Development => test_dialect!(development, &args.address),
        #[cfg(feature = "matrixpilot")]
        Dialect::Matrixpilot => test_dialect!(matrixpilot, &args.address),
        #[cfg(feature = "paparazzi")]
        Dialect::Paparazzi => test_dialect!(paparazzi, &args.address),
        #[cfg(feature = "storm32")]
        Dialect::Storm32 => test_dialect!(storm32, &args.address),
        #[cfg(feature = "u_avionix")]
        Dialect::UAvionix => test_dialect!(u_avionix, &args.address),
        #[cfg(feature = "ualberta")]
        Dialect::Ualberta => test_dialect!(ualberta, &args.address),
    }
}
