#![allow(clippy::needless_late_init)] // used by some generated code

use hard_xml::XmlRead;

#[derive(XmlRead, Clone, PartialEq, Debug)]
#[xml(tag = "description")]
pub struct Description(#[xml(text)] pub String);

impl From<&str> for Description {
    fn from(value: &str) -> Self {
        Description(value.to_owned())
    }
}

impl Description {
    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(XmlRead, Clone, PartialEq, Debug)]
#[xml(tag = "deprecated")]
pub struct Deprecated {
    // In the mavschema.xsd, this is marked as a child element. However
    // the mavlink schemas specify description inline, so let's also use it
    // this way
    //
    // Also, cannot make this Option<Cow>> because hard_xml always puts an empty
    // string in case this is not defined.
    #[xml(text)]
    pub description: String,

    #[xml(attr = "since")]
    pub since: String,

    #[xml(attr = "replaced_by")]
    pub replaced_by: String,
}

#[derive(XmlRead, Clone, PartialEq, Debug)]
#[xml(tag = "wip")]
pub struct Wip {
    // Why `text` and not `child` - see the comment for Deprecated::description.
    // Although, in this case there is no example with WIP description in the
    // official mavlink definitions.
    #[xml(text)]
    pub description: String,

    #[xml(attr = "since")]
    pub since: Option<String>,
}

#[derive(XmlRead, Clone, PartialEq, Debug)]
#[xml(tag = "deprecated")]
pub enum DevStatus {
    #[xml(tag = "deprecated")]
    Deprecated(Deprecated),

    #[xml(tag = "wip")]
    Wip(Wip),
}

#[derive(XmlRead, Clone, PartialEq, Debug)]
#[xml(tag = "param")]
pub struct Param {
    #[xml(attr = "index")]
    pub index: u8,

    #[xml(attr = "label")]
    pub label: Option<String>,

    #[xml(attr = "units")]
    pub units: Option<String>,

    #[xml(attr = "multiplier")]
    pub multiplier: Option<String>,

    #[xml(attr = "instance")]
    pub instance: Option<bool>,

    #[xml(attr = "enum")]
    pub r#enum: Option<String>,

    #[xml(attr = "decimalPlaces")]
    pub decimal_places: Option<u8>,

    #[xml(attr = "increment")]
    pub increment: Option<f32>,

    #[xml(attr = "minValue")]
    pub min_value: Option<f32>,

    #[xml(attr = "maxValue")]
    pub max_value: Option<f32>,

    #[xml(attr = "reserved")]
    pub reserved: Option<bool>,

    #[xml(attr = "default")]
    pub default: Option<String>,

    #[xml(text)]
    pub content: Option<String>,
}

#[derive(XmlRead, Clone, PartialEq, Debug)]
#[xml(tag = "entry")]
pub struct Entry {
    #[xml(attr = "name")]
    pub name: String,

    #[xml(attr = "value")]
    pub value: Option<String>,

    #[xml(attr = "hasLocation")]
    pub has_location: Option<bool>,

    #[xml(attr = "isDestination")]
    pub is_destination: Option<bool>,

    #[xml(attr = "missionOnly")]
    pub mission_only: Option<bool>,

    #[xml(child = "description")]
    pub description: Option<Description>,

    #[xml(child = "param")]
    pub params: Vec<Param>,

    #[xml(child = "deprecated", child = "wip")]
    pub dev_status: Option<DevStatus>,
}

impl Entry {
    #[cfg(test)]
    pub fn new_min(name: impl Into<String>, value: Option<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            value: value.map(Into::into),
            has_location: None,
            is_destination: None,
            mission_only: None,
            description: None,
            params: vec![],
            dev_status: None,
        }
    }
}

#[derive(XmlRead, Clone, PartialEq, Debug)]
#[xml(tag = "enum")]
pub struct Enum {
    #[xml(attr = "name")]
    pub name: String,

    #[xml(attr = "bitmask")]
    pub bitmask: Option<bool>,

    #[xml(child = "description")]
    pub description: Option<Description>,

    #[xml(child = "deprecated", child = "wip")]
    pub dev_status: Option<DevStatus>,

    #[xml(child = "entry")]
    pub entries: Vec<Entry>,
}

#[derive(XmlRead, Clone, PartialEq, Debug)]
#[xml(tag = "enums")]
pub struct Enums(#[xml(child = "enum")] pub Vec<Enum>);

#[derive(XmlRead, Clone, PartialEq, Debug)]
#[xml(tag = "field")]
pub struct Field {
    #[xml(attr = "name")]
    pub name: String,

    #[xml(attr = "type")]
    pub r#type: String,

    #[xml(attr = "print_format")]
    pub print_format: Option<String>,

    #[xml(attr = "enum")]
    pub r#enum: Option<String>,

    #[xml(attr = "display")]
    pub display: Option<String>,

    #[xml(attr = "units")]
    pub units: Option<String>,

    #[xml(attr = "increment")]
    pub increment: Option<f32>,

    #[xml(attr = "minValue")]
    pub min_value: Option<f32>,

    #[xml(attr = "maxValue")]
    pub max_value: Option<f32>,

    #[xml(attr = "multiplier")]
    pub multiplier: Option<String>,

    #[xml(attr = "default")]
    pub default: Option<String>,

    #[xml(attr = "instance")]
    pub instance: Option<bool>,

    #[xml(attr = "invalid")]
    pub invalid: Option<String>,

    // Even though in mavshema this field is marked as child element, in reality
    // the description is provided as a text in the `field` element itself.
    #[xml(text)]
    pub description: String,
}

impl Field {
    #[cfg(test)]
    pub fn new_min(name: impl Into<String>, r#type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            r#type: r#type.into(),
            print_format: None,
            r#enum: None,
            display: None,
            units: None,
            increment: None,
            min_value: None,
            max_value: None,
            multiplier: None,
            default: None,
            instance: None,
            invalid: None,
            description: String::new(),
        }
    }
}

#[derive(XmlRead, Clone, PartialEq, Debug)]
#[xml(tag = "extensions")]
pub struct Extensions;

#[derive(PartialEq, Debug, Clone)]
pub struct Message {
    pub name: String,
    pub id: u32,
    pub dev_status: Option<DevStatus>,
    pub description: Option<Description>,
    pub fields: Vec<Field>,
    pub extension_fields: Vec<Field>,
}

// Derive manually, because mavlink message definitions have weird field
// pub structure, where all fields after the `extensions` tag are extension fields.
// hard_xml doesn't care about field ordering, so we should do it manually.
impl<'input: 'a, 'a> XmlRead<'input> for Message {
    fn from_reader(reader: &mut hard_xml::XmlReader<'input>) -> hard_xml::XmlResult<Self> {
        use hard_xml::xmlparser::{ElementEnd, Token};
        use hard_xml::XmlError;
        let mut name = None;
        let mut id = None;
        let mut dev_status = None;
        let mut description = None;
        let mut fields = Vec::new();
        let mut extension_fields = Vec::new();
        reader.read_till_element_start("message")?;
        while let Some((key, value)) = reader.find_attribute()? {
            match key {
                "name" => {
                    name = Some(value);
                }
                "id" => {
                    id = Some(
                        <u32 as std::str::FromStr>::from_str(&value)
                            .map_err(|e| XmlError::FromStr(e.into()))?,
                    );
                }
                _ => {}
            }
        }
        if let Token::ElementEnd {
            end: ElementEnd::Empty,
            ..
        } = reader.next().unwrap()?
        {
            return Ok(Message {
                name: name
                    .ok_or(XmlError::MissingField {
                        name: "Message".to_owned(),
                        field: "name".to_owned(),
                    })?
                    .into_owned(),
                id: id.ok_or(XmlError::MissingField {
                    name: "Message".to_owned(),
                    field: "id".to_owned(),
                })?,
                dev_status,
                description,
                fields,
                extension_fields,
            });
        }

        let mut are_extension_fields = false;

        while let Some(tag) = reader.find_element_start(Some("message"))? {
            match tag {
                "deprecated" | "wip" => {
                    dev_status = Some(<DevStatus as hard_xml::XmlRead>::from_reader(reader)?);
                }
                "description" => {
                    description = Some(<Description as hard_xml::XmlRead>::from_reader(reader)?);
                }
                "field" => {
                    if are_extension_fields {
                        extension_fields.push(<Field as hard_xml::XmlRead>::from_reader(reader)?);
                    } else {
                        fields.push(<Field as hard_xml::XmlRead>::from_reader(reader)?);
                    }
                }
                "extensions" => {
                    Extensions::from_reader(reader)?;
                    are_extension_fields = true;
                }
                tag => {
                    reader.next();
                    reader.read_to_end(tag)?;
                }
            }
        }
        Ok(Message {
            name: name
                .ok_or(XmlError::MissingField {
                    name: "Message".to_owned(),
                    field: "name".to_owned(),
                })?
                .into_owned(),
            id: id.ok_or(XmlError::MissingField {
                name: "Message".to_owned(),
                field: "id".to_owned(),
            })?,
            dev_status,
            description,
            fields,
            extension_fields,
        })
    }
}

#[derive(XmlRead, Clone, PartialEq, Debug)]
#[xml(tag = "messages")]
pub struct Messages(#[xml(child = "message")] pub Vec<Message>);

#[derive(XmlRead, Clone, PartialEq, Debug)]
#[xml(tag = "mavlink")]
pub struct Mavlink {
    #[xml(flatten_text = "include")]
    pub include: Vec<String>,

    #[xml(flatten_text = "version")]
    pub version: Option<u8>,

    #[xml(flatten_text = "dialect")]
    pub dialect: Option<u8>,

    #[xml(child = "enums")]
    pub enums: Option<Enums>,

    #[xml(child = "messages")]
    pub messages: Option<Messages>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_param_smoke() {
        let raw = r#"
        <param index="1" label="ID" minValue="0" maxValue="65535" increment="1">uint16 ID value to be passed to scripting</param>
        "#;

        let parsed = Param::from_str(raw).unwrap();
        let expected = Param {
            index: 1,
            label: Some(String::from("ID")),
            units: None,
            multiplier: None,
            instance: None,
            r#enum: None,
            decimal_places: None,
            increment: Some(1.0),
            min_value: Some(0.0),
            max_value: Some(65535.0),
            reserved: None,
            default: None,
            content: Some(String::from("uint16 ID value to be passed to scripting")),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_param_smoke_2() {
        let raw = r#"
        <param index="2" label="param 1">float value to be passed to scripting</param>
        "#;

        let parsed = Param::from_str(raw).unwrap();
        let expected = Param {
            index: 2,
            label: Some(String::from("param 1")),
            units: None,
            multiplier: None,
            instance: None,
            r#enum: None,
            decimal_places: None,
            increment: None,
            min_value: None,
            max_value: None,
            reserved: None,
            default: None,
            content: Some(String::from("float value to be passed to scripting")),
        };
        assert_eq!(parsed, expected);

        let raw = r#"
        <param index="3" label="Intensity" units="mgauss">Magnetic intensity.</param>
        "#;

        let parsed = Param::from_str(raw).unwrap();
        let expected = Param {
            index: 3,
            label: Some(String::from("Intensity")),
            units: Some(String::from("mgauss")),
            multiplier: None,
            instance: None,
            r#enum: None,
            decimal_places: None,
            increment: None,
            min_value: None,
            max_value: None,
            reserved: None,
            default: None,
            content: Some(String::from("Magnetic intensity.")),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_param_all_attrs() {
        let raw = r#"
        <param index="7" label="all attrs" units="cats" multiplier="100x" instance="true" enum="super enum" decimalPlaces="10" increment="1" maxValue="255" minValue="0" reserved="true" default="turbofish"
        >!!!!</param>
        "#;

        let parsed = Param::from_str(raw).unwrap();
        let expected = Param {
            index: 7,
            label: Some(String::from("all attrs")),
            units: Some(String::from("cats")),
            multiplier: Some(String::from("100x")),
            instance: Some(true),
            r#enum: Some(String::from("super enum")),
            decimal_places: Some(10),
            increment: Some(1.0),
            min_value: Some(0.0),
            max_value: Some(255.0),
            reserved: Some(true),
            default: Some(String::from("turbofish")),
            content: Some(String::from("!!!!")),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_entry_smoke() {
        let raw = r#"<entry value="0" name="OSD_PARAM_NONE"/>"#;

        let parsed = Entry::from_str(raw).unwrap();
        let expected = Entry {
            name: String::from("OSD_PARAM_NONE"),
            value: Some(String::from("0")),
            has_location: None,
            is_destination: None,
            mission_only: None,
            description: None,
            params: vec![],
            dev_status: None,
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_entry_mav_cmd() {
        let raw = r#"
        <entry value="42426" name="MAV_CMD_DO_CANCEL_MAG_CAL" hasLocation="false" isDestination="false">
            <description>Cancel a running magnetometer calibration.</description>
            <param index="1" label="Magnetometers Bitmask" minValue="0" maxValue="255" increment="1">Bitmask of magnetometers to cancel a running calibration (0 means all).</param>
            <param index="2">Empty.</param>
            <param index="3">Empty.</param>
            <param index="4">Empty.</param>
            <param index="5">Empty.</param>
            <param index="6">Empty.</param>
            <param index="7">Empty.</param>
        </entry>
        "#;

        let empty_param = Param {
            index: 0,
            label: None,
            units: None,
            multiplier: None,
            instance: None,
            r#enum: None,
            decimal_places: None,
            increment: None,
            min_value: None,
            max_value: None,
            reserved: None,
            default: None,
            content: Some(String::from("Empty.")),
        };

        let parsed = Entry::from_str(raw).unwrap();
        let expected = Entry {
            name: String::from("MAV_CMD_DO_CANCEL_MAG_CAL"),
            value: Some(String::from("42426")),
            has_location: Some(false),
            is_destination: Some(false),
            mission_only: None,
            description: Some(Description::from(
                "Cancel a running magnetometer calibration.",
            )),
            params: vec![
                Param {
                    index: 1,
                    label: Some(String::from("Magnetometers Bitmask")),
                    min_value: Some(0.0),
                    max_value: Some(255.0),
                    increment: Some(1.0),
                    content: Some(String::from(
                        "Bitmask of magnetometers to cancel a running calibration (0 means all).",
                    )),
                    ..empty_param.clone()
                },
                Param {
                    index: 2,
                    ..empty_param.clone()
                },
                Param {
                    index: 3,
                    ..empty_param.clone()
                },
                Param {
                    index: 4,
                    ..empty_param.clone()
                },
                Param {
                    index: 5,
                    ..empty_param.clone()
                },
                Param {
                    index: 6,
                    ..empty_param.clone()
                },
                Param {
                    index: 7,
                    ..empty_param.clone()
                },
            ],

            dev_status: None,
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_dev_status() {
        let raw = r#"
        <deprecated since="2020-01" replaced_by="MAV_CMD_DO_GIMBAL_MANAGER_CONFIGURE">This message has been superseded by MAV_CMD_DO_GIMBAL_MANAGER_CONFIGURE. The message can still be used to communicate with legacy gimbals implementing it.</deprecated>
        "#;

        let parsed = DevStatus::from_str(raw).unwrap();
        let expected = DevStatus::Deprecated(Deprecated{
            description: String::from("This message has been superseded by MAV_CMD_DO_GIMBAL_MANAGER_CONFIGURE. The message can still be used to communicate with legacy gimbals implementing it."),
            since: String::from("2020-01"),
            replaced_by: String::from("MAV_CMD_DO_GIMBAL_MANAGER_CONFIGURE"),
        });
        assert_eq!(parsed, expected);

        let raw = r#"
        <deprecated since="2020-02" replaced_by="MAV_CMD_DO_GIMBAL_MANAGER_CONFIGURE"/>
        "#;

        let parsed = DevStatus::from_str(raw).unwrap();
        let expected = DevStatus::Deprecated(Deprecated {
            description: String::from(""),
            since: String::from("2020-02"),
            replaced_by: String::from("MAV_CMD_DO_GIMBAL_MANAGER_CONFIGURE"),
        });
        assert_eq!(parsed, expected);

        // since is mandatory
        let raw = r#"
        <deprecated replaced_by="MAV_CMD_DO_GIMBAL_MANAGER_CONFIGURE"/>
        "#;
        DevStatus::from_str(raw).unwrap_err();

        // replaced_by is also mandatory
        let raw = r#"
        <deprecated since="2020-02"/>
        "#;
        DevStatus::from_str(raw).unwrap_err();

        let raw = r#"
        <wip since="2020-02"/>
        "#;

        let parsed = DevStatus::from_str(raw).unwrap();
        let expected = DevStatus::Wip(Wip {
            since: Some(String::from("2020-02")),
            description: String::from(""),
        });
        assert_eq!(parsed, expected);

        let raw = r#"
        <wip>Wizard in Programming</wip>
        "#;

        let parsed = DevStatus::from_str(raw).unwrap();
        let expected = DevStatus::Wip(Wip {
            since: None,
            description: String::from("Wizard in Programming"),
        });
        assert_eq!(parsed, expected);

        let raw = r#"
        <wip/>
        "#;

        let parsed = DevStatus::from_str(raw).unwrap();
        let expected = DevStatus::Wip(Wip {
            since: None,
            description: String::from(""),
        });
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_entry_all_fields() {
        let raw = r#"
        <entry
            value="12345"
            name="SOME_CONSTANT"
            hasLocation="true"
            isDestination="false"
            missionOnly="true"
        >
            <deprecated since="2014-07" replaced_by="SOME_OTHER_CONSTANT">Use other constant.</deprecated>
            <description>Helpful readme.</description>
            <param index="1">Empty.</param>
        </entry>
        "#;

        let parsed = Entry::from_str(raw).unwrap();
        let expected = Entry {
            name: String::from("SOME_CONSTANT"),
            value: Some(String::from("12345")),
            has_location: Some(true),
            is_destination: Some(false),
            mission_only: Some(true),
            description: Some(Description::from("Helpful readme.")),
            params: vec![Param {
                index: 1,
                label: None,
                units: None,
                multiplier: None,
                instance: None,
                r#enum: None,
                decimal_places: None,
                increment: None,
                min_value: None,
                max_value: None,
                reserved: None,
                default: None,
                content: Some(String::from("Empty.")),
            }],

            dev_status: Some(DevStatus::Deprecated(Deprecated {
                description: String::from("Use other constant."),
                since: String::from("2014-07"),
                replaced_by: String::from("SOME_OTHER_CONSTANT"),
            })),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_entry_wip() {
        let raw = r#"
        <entry value="34" name="MAV_CMD_DO_ORBIT" hasLocation="true" isDestination="true">
            <wip/>
            <!-- This message is work-in-progress and it can therefore change. It should NOT be used in stable production environments. -->
            <description>Start orbiting on the circumference of a circle defined by the parameters. Setting values to NaN/INT32_MAX (as appropriate) results in using defaults.</description>
        </entry>
        "#;

        let parsed = Entry::from_str(raw).unwrap();
        let expected = Entry {
            name: String::from("MAV_CMD_DO_ORBIT"),
            value: Some(String::from("34")),
            has_location: Some(true),
            is_destination: Some(true),
            mission_only: None,
            description: Some(Description::from("Start orbiting on the circumference of a circle defined by the parameters. Setting values to NaN/INT32_MAX (as appropriate) results in using defaults.")),
            params: vec![],

            dev_status: Some(DevStatus::Wip(Wip { description: String::from(""), since: None })),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_enum_smoke() {
        let raw = r#"
        <enum name="ACCELCAL_VEHICLE_POS">
            <entry value="1" name="ACCELCAL_VEHICLE_POS_LEVEL"/>
            <entry value="2" name="ACCELCAL_VEHICLE_POS_LEFT"/>
            <entry value="3" name="ACCELCAL_VEHICLE_POS_RIGHT"/>
            <entry value="4" name="ACCELCAL_VEHICLE_POS_NOSEDOWN"/>
            <entry value="5" name="ACCELCAL_VEHICLE_POS_NOSEUP"/>
            <entry value="6" name="ACCELCAL_VEHICLE_POS_BACK"/>
            <entry value="16777215" name="ACCELCAL_VEHICLE_POS_SUCCESS"/>
            <entry value="16777216" name="ACCELCAL_VEHICLE_POS_FAILED"/>
        </enum>
        "#;

        let parsed = Enum::from_str(raw).unwrap();
        let expected = Enum {
            name: String::from("ACCELCAL_VEHICLE_POS"),
            bitmask: None,
            entries: vec![
                Entry::new_min("ACCELCAL_VEHICLE_POS_LEVEL", Some("1")),
                Entry::new_min("ACCELCAL_VEHICLE_POS_LEFT", Some("2")),
                Entry::new_min("ACCELCAL_VEHICLE_POS_RIGHT", Some("3")),
                Entry::new_min("ACCELCAL_VEHICLE_POS_NOSEDOWN", Some("4")),
                Entry::new_min("ACCELCAL_VEHICLE_POS_NOSEUP", Some("5")),
                Entry::new_min("ACCELCAL_VEHICLE_POS_BACK", Some("6")),
                Entry::new_min("ACCELCAL_VEHICLE_POS_SUCCESS", Some("16777215")),
                Entry::new_min("ACCELCAL_VEHICLE_POS_FAILED", Some("16777216")),
            ],
            dev_status: None,
            description: None,
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_enum_bitflag() {
        let raw = r#"
        <enum name="RALLY_FLAGS" bitmask="true">
            <description>Flags in RALLY_POINT message.</description>
            <entry value="1" name="FAVORABLE_WIND">
                <description>Flag set when requiring favorable winds for landing.</description>
            </entry>
            <entry value="2" name="LAND_IMMEDIATELY">
                <description>Flag set when plane is to immediately descend to break altitude and land without GCS intervention. Flag not set when plane is to loiter at Rally point until commanded to land.</description>
            </entry>
        </enum>
        "#;

        let parsed = Enum::from_str(raw).unwrap();

        let expected = Enum {
            name: String::from("RALLY_FLAGS"),
            bitmask: Some(true),
            entries: vec![
                Entry {
                    name: String::from("FAVORABLE_WIND"),
                    value: Some(String::from("1")),
                    has_location: None,
                    is_destination: None,
                    mission_only: None,
                    description: Some(Description::from("Flag set when requiring favorable winds for landing.")),
                    params: vec![],
                    dev_status: None
                },
                Entry {
                    name: String::from("LAND_IMMEDIATELY"),
                    value: Some(String::from("2")),
                    has_location: None,
                    is_destination: None,
                    mission_only: None,
                    description: Some(Description::from("Flag set when plane is to immediately descend to break altitude and land without GCS intervention. Flag not set when plane is to loiter at Rally point until commanded to land.")),
                    params: vec![],
                    dev_status: None
                }
            ],
            dev_status: None,
            description: Some(Description::from("Flags in RALLY_POINT message.")),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_enum_dev_status() {
        let raw = r#"
        <enum name="SOME_ENUM" bitmask="false">
            <wip/>
        </enum>
        "#;

        let parsed = Enum::from_str(raw).unwrap();

        let expected = Enum {
            name: String::from("SOME_ENUM"),
            bitmask: Some(false),
            entries: vec![],
            dev_status: Some(DevStatus::Wip(Wip {
                description: String::from(""),
                since: None,
            })),
            description: None,
        };
        assert_eq!(parsed, expected);

        let raw = r#"
        <enum name="SOME_ENUM" bitmask="false">
            <deprecated since="2024-08" replaced_by="SOME_OTHER_ENUM"/>
        </enum>
        "#;

        let parsed = Enum::from_str(raw).unwrap();

        let expected = Enum {
            name: String::from("SOME_ENUM"),
            bitmask: Some(false),
            entries: vec![],
            dev_status: Some(DevStatus::Deprecated(Deprecated {
                description: String::from(""),
                since: String::from("2024-08"),
                replaced_by: String::from("SOME_OTHER_ENUM"),
            })),
            description: None,
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_enums() {
        let raw = r#"
          <enums>
            <enum name="UAVIONIX_ADSB_OUT_DYNAMIC_STATE" bitmask="true">
                <description>State flags for ADS-B transponder dynamic report</description>
                <entry value="1" name="UAVIONIX_ADSB_OUT_DYNAMIC_STATE_INTENT_CHANGE"/>
                <entry value="2" name="UAVIONIX_ADSB_OUT_DYNAMIC_STATE_AUTOPILOT_ENABLED"/>
                <entry value="4" name="UAVIONIX_ADSB_OUT_DYNAMIC_STATE_NICBARO_CROSSCHECKED"/>
                <entry value="8" name="UAVIONIX_ADSB_OUT_DYNAMIC_STATE_ON_GROUND"/>
                <entry value="16" name="UAVIONIX_ADSB_OUT_DYNAMIC_STATE_IDENT"/>
            </enum>
            <enum name="UAVIONIX_ADSB_OUT_RF_SELECT" bitmask="true">
                <description>Transceiver RF control flags for ADS-B transponder dynamic reports</description>
                <entry value="0" name="UAVIONIX_ADSB_OUT_RF_SELECT_STANDBY"/>
                <entry value="1" name="UAVIONIX_ADSB_OUT_RF_SELECT_RX_ENABLED"/>
                <entry value="2" name="UAVIONIX_ADSB_OUT_RF_SELECT_TX_ENABLED"/>
            </enum>
            <enum name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX">
                <description>Status for ADS-B transponder dynamic input</description>
                <entry value="0" name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_NONE_0"/>
                <entry value="1" name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_NONE_1"/>
                <entry value="2" name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_2D"/>
                <entry value="3" name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_3D"/>
                <entry value="4" name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_DGPS"/>
                <entry value="5" name="UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_RTK"/>
            </enum>
        </enums>
        "#;

        let parsed = Enums::from_str(raw).unwrap();
        let expected = Enums(vec![
            Enum {
                name: String::from("UAVIONIX_ADSB_OUT_DYNAMIC_STATE"),
                bitmask: Some(true),
                entries: vec![
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_DYNAMIC_STATE_INTENT_CHANGE"),
                        value: Some(String::from("1")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_DYNAMIC_STATE_AUTOPILOT_ENABLED"),
                        value: Some(String::from("2")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_DYNAMIC_STATE_NICBARO_CROSSCHECKED"),
                        value: Some(String::from("4")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_DYNAMIC_STATE_ON_GROUND"),
                        value: Some(String::from("8")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_DYNAMIC_STATE_IDENT"),
                        value: Some(String::from("16")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                ],
                dev_status: None,
                description: Some(Description(String::from(
                    "State flags for ADS-B transponder dynamic report",
                ))),
            },
            Enum {
                name: String::from("UAVIONIX_ADSB_OUT_RF_SELECT"),
                bitmask: Some(true),
                entries: vec![
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_RF_SELECT_STANDBY"),
                        value: Some(String::from("0")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_RF_SELECT_RX_ENABLED"),
                        value: Some(String::from("1")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_RF_SELECT_TX_ENABLED"),
                        value: Some(String::from("2")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                ],
                dev_status: None,
                description: Some(Description(String::from(
                    "Transceiver RF control flags for ADS-B transponder dynamic reports",
                ))),
            },
            Enum {
                name: String::from("UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX"),
                bitmask: None,
                entries: vec![
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_NONE_0"),
                        value: Some(String::from("0")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_NONE_1"),
                        value: Some(String::from("1")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_2D"),
                        value: Some(String::from("2")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_3D"),
                        value: Some(String::from("3")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_DGPS"),
                        value: Some(String::from("4")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                    Entry {
                        name: String::from("UAVIONIX_ADSB_OUT_DYNAMIC_GPS_FIX_RTK"),
                        value: Some(String::from("5")),
                        has_location: None,
                        is_destination: None,
                        mission_only: None,
                        description: None,
                        params: vec![],
                        dev_status: None,
                    },
                ],
                dev_status: None,
                description: Some(Description(String::from(
                    "Status for ADS-B transponder dynamic input",
                ))),
            },
        ]);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_field_smoke() {
        let raw = r#"
        <field type="float" name="diff_pressure" units="Pa">Differential pressure.</field>
        "#;

        let parsed = Field::from_str(raw).unwrap();

        let expected = Field {
            name: String::from("diff_pressure"),
            r#type: String::from("float"),
            print_format: None,
            r#enum: None,
            display: None,
            units: Some(String::from("Pa")),
            increment: None,
            min_value: None,
            max_value: None,
            multiplier: None,
            default: None,
            instance: None,
            invalid: None,
            description: String::from("Differential pressure."),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_field_smoke_bitfield() {
        let raw = r#"
        <field type="uint8_t" name="mods_required" enum="LIMIT_MODULE" display="bitmask">AP_Limit_Module bitfield of required modules.</field>
        "#;

        let parsed = Field::from_str(raw).unwrap();

        let expected = Field {
            name: String::from("mods_required"),
            r#type: String::from("uint8_t"),
            print_format: None,
            r#enum: Some(String::from("LIMIT_MODULE")),
            display: Some(String::from("bitmask")),
            units: None,
            increment: None,
            min_value: None,
            max_value: None,
            multiplier: None,
            default: None,
            instance: None,
            invalid: None,
            description: String::from("AP_Limit_Module bitfield of required modules."),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_field_without_description() {
        let raw = r#"
        <field type="int16_t" name="board_temp"/>
        "#;

        let parsed = Field::from_str(raw).unwrap();

        let expected = Field {
            name: String::from("board_temp"),
            r#type: String::from("int16_t"),
            print_format: None,
            r#enum: None,
            display: None,
            units: None,
            increment: None,
            min_value: None,
            max_value: None,
            multiplier: None,
            default: None,
            instance: None,
            invalid: None,
            description: String::from(""),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_field_all_attributes() {
        let raw = r#"
        <field
            type="uint32_t"
            name="onboard_control_sensors_enabled"
            enum="MAV_SYS_STATUS_SENSOR"
            display="bitmask"
            print_format="0x%04x"
            units="cats"
            increment="1"
            minValue="-1"
            maxValue="10"
            multiplier="1E-2"
            default="0"
            instance="true"
            invalid="UINT16_MAX"
        >Bitmap showing which onboard controllers and sensors are enabled:  Value of 0: not enabled. Value of 1: enabled.</field>
        "#;

        let parsed = Field::from_str(raw).unwrap();

        let expected = Field {
            name: String::from("onboard_control_sensors_enabled"),
            r#type: String::from("uint32_t"),
            print_format: Some(String::from("0x%04x")),
            r#enum: Some(String::from("MAV_SYS_STATUS_SENSOR")),
            display: Some(String::from("bitmask")),
            units: Some(String::from("cats")),
            increment: Some(1.0),
            min_value: Some(-1.0),
            max_value: Some(10.0),
            multiplier: Some(String::from("1E-2")),
            default: Some(String::from("0")),
            instance: Some(true),
            invalid: Some(String::from("UINT16_MAX")),
            description: String::from("Bitmap showing which onboard controllers and sensors are enabled:  Value of 0: not enabled. Value of 1: enabled."),
        };
        assert_eq!(parsed, expected);
    }

    const DEFAULT_FIELD: Field = Field {
        name: String::new(),
        r#type: String::new(),
        print_format: None,
        r#enum: None,
        display: None,
        units: None,
        increment: None,
        min_value: None,
        max_value: None,
        multiplier: None,
        default: None,
        instance: None,
        invalid: None,
        description: String::new(),
    };

    #[test]
    fn test_message_smoke_test() {
        let raw = r#"
        <message id="169" name="DATA16">
            <description>Data packet, size 16.</description>
            <field type="uint8_t" name="type">Data type.</field>
            <field type="uint8_t" name="len" units="bytes">Data length.</field>
            <field type="uint8_t[16]" name="data">Raw data.</field>
        </message>
        "#;

        let parsed = Message::from_str(raw).unwrap();

        let expected = Message {
            name: String::from("DATA16"),
            id: 169,
            dev_status: None,
            description: Some(Description::from("Data packet, size 16.")),
            fields: vec![
                Field {
                    name: String::from("type"),
                    r#type: String::from("uint8_t"),
                    description: String::from("Data type."),
                    ..DEFAULT_FIELD.clone()
                },
                Field {
                    name: String::from("len"),
                    r#type: String::from("uint8_t"),
                    units: Some(String::from("bytes")),
                    description: String::from("Data length."),
                    ..DEFAULT_FIELD.clone()
                },
                Field {
                    name: String::from("data"),
                    r#type: String::from("uint8_t[16]"),
                    description: String::from("Raw data."),
                    ..DEFAULT_FIELD.clone()
                },
            ],
            extension_fields: vec![],
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_message_extension_fields() {
        let raw = r#"
        <message id="100" name="OPTICAL_FLOW">
            <description>Optical flow from a flow sensor (e.g. optical mouse sensor)</description>
            <field type="uint64_t" name="time_usec" units="us">Timestamp (UNIX Epoch time or time since system boot). The receiving end can infer timestamp format (since 1.1.1970 or since system boot) by checking for the magnitude of the number.</field>
            <field type="uint8_t" name="sensor_id">Sensor ID</field>
            <field type="int16_t" name="flow_x" units="dpix">Flow in x-sensor direction</field>
            <field type="int16_t" name="flow_y" units="dpix">Flow in y-sensor direction</field>
            <field type="float" name="flow_comp_m_x" units="m/s">Flow in x-sensor direction, angular-speed compensated</field>
            <field type="float" name="flow_comp_m_y" units="m/s">Flow in y-sensor direction, angular-speed compensated</field>
            <field type="uint8_t" name="quality">Optical flow quality / confidence. 0: bad, 255: maximum quality</field>
            <field type="float" name="ground_distance" units="m">Ground distance. Positive value: distance known. Negative value: Unknown distance</field>
            <extensions/>
            <field type="float" name="flow_rate_x" units="rad/s">Flow rate about X axis</field>
            <field type="float" name="flow_rate_y" units="rad/s">Flow rate about Y axis</field>
        </message>
        "#;

        let parsed = Message::from_str(raw).unwrap();

        let expected = Message {
            name: String::from("OPTICAL_FLOW"),
            id: 100,
            dev_status: None,
            description: Some(
                Description(
                    String::from("Optical flow from a flow sensor (e.g. optical mouse sensor)"),
                ),
            ),
            fields: vec![
                Field {
                    name: String::from("time_usec"),
                    r#type: String::from("uint64_t"),
                    units: Some(String::from("us")),
                    description: String::from("Timestamp (UNIX Epoch time or time since system boot). The receiving end can infer timestamp format (since 1.1.1970 or since system boot) by checking for the magnitude of the number."),
                    ..DEFAULT_FIELD.clone()
                },
                Field {
                    name: String::from("sensor_id"),
                    r#type: String::from("uint8_t"),
                    description: String::from("Sensor ID"),
                    ..DEFAULT_FIELD.clone()

                },
                Field {
                    name: String::from("flow_x"),
                    r#type: String::from("int16_t"),
                    units: Some(String::from("dpix")),
                    description: String::from("Flow in x-sensor direction"),
                    ..DEFAULT_FIELD.clone()
                },
                Field {
                    name: String::from("flow_y"),
                    r#type: String::from("int16_t"),
                    units: Some(String::from("dpix")),
                    description: String::from("Flow in y-sensor direction"),
                    ..DEFAULT_FIELD.clone()
                },
                Field {
                    name: String::from("flow_comp_m_x"),
                    r#type: String::from("float"),
                    units: Some(String::from("m/s")),
                    description: String::from("Flow in x-sensor direction, angular-speed compensated"),
                    ..DEFAULT_FIELD.clone()
                },
                Field {
                    name: String::from("flow_comp_m_y"),
                    r#type: String::from("float"),
                    units: Some(String::from("m/s")),
                    description: String::from("Flow in y-sensor direction, angular-speed compensated"),
                    ..DEFAULT_FIELD.clone()
                },
                Field {
                    name: String::from("quality"),
                    r#type: String::from("uint8_t"),
                    description: String::from("Optical flow quality / confidence. 0: bad, 255: maximum quality"),
                    ..DEFAULT_FIELD.clone()
                },
                Field {
                    name: String::from("ground_distance"),
                    r#type: String::from("float"),
                    units: Some(String::from("m")),
                    description: String::from("Ground distance. Positive value: distance known. Negative value: Unknown distance"),
                    ..DEFAULT_FIELD.clone()
                },
            ],
            extension_fields: vec![
                Field {
                    name: String::from("flow_rate_x"),
                    r#type: String::from("float"),
                    units: Some(String::from("rad/s")),
                    description: String::from("Flow rate about X axis"),
                    ..DEFAULT_FIELD.clone()
                },
                Field {
                    name: String::from("flow_rate_y"),
                    r#type: String::from("float"),
                    units: Some(String::from("rad/s")),
                    description: String::from("Flow rate about Y axis"),
                    ..DEFAULT_FIELD.clone()
                },
            ],
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_message_all_fields() {
        let raw = r#"
        <message id="40" name="MISSION_REQUEST">
            <deprecated since="2020-06" replaced_by="MISSION_REQUEST_INT">A system that gets this request should respond with MISSION_ITEM_INT (as though MISSION_REQUEST_INT was received).</deprecated>
            <description>Request the information of the mission item with the sequence number seq. The response of the system to this message should be a MISSION_ITEM message. https://mavlink.io/en/services/mission.html</description>
            <field type="uint8_t" name="target_system">System ID</field>
            <extensions/>
            <field type="uint8_t" name="mission_type" enum="MAV_MISSION_TYPE">Mission type.</field>
        </message>
        "#;

        let parsed = Message::from_str(raw).unwrap();

        let expected = Message {
            name: String::from("MISSION_REQUEST"),
            id: 40,
            dev_status: Some(
                DevStatus::Deprecated(
                    Deprecated {
                        description: String::from("A system that gets this request should respond with MISSION_ITEM_INT (as though MISSION_REQUEST_INT was received)."),
                        since: String::from("2020-06"),
                        replaced_by: String::from("MISSION_REQUEST_INT"),
                    },
                ),
            ),
            description: Some(
                Description(
                    String::from("Request the information of the mission item with the sequence number seq. The response of the system to this message should be a MISSION_ITEM message. https://mavlink.io/en/services/mission.html"),
                ),
            ),
            fields: vec![
                Field {
                    name: String::from("target_system"),
                    r#type: String::from("uint8_t"),
                    print_format: None,
                    r#enum: None,
                    display: None,
                    units: None,
                    increment: None,
                    min_value: None,
                    max_value: None,
                    multiplier: None,
                    default: None,
                    instance: None,
                    invalid: None,
                    description: String::from("System ID"),
                },
            ],
            extension_fields: vec![
                Field {
                    name: String::from("mission_type"),
                    r#type: String::from("uint8_t"),
                    print_format: None,
                    r#enum: Some(
                        String::from("MAV_MISSION_TYPE"),
                    ),
                    display: None,
                    units: None,
                    increment: None,
                    min_value: None,
                    max_value: None,
                    multiplier: None,
                    default: None,
                    instance: None,
                    invalid: None,
                    description: String::from("Mission type."),
                },
            ],
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_message_wip() {
        let raw = r#"
        <message id="40" name="MISSION_REQUEST">
            <wip/>
            <field type="uint8_t" name="target_system">System ID</field>
        </message>
        "#;

        let parsed = Message::from_str(raw).unwrap();
        let expected = Message {
            name: String::from("MISSION_REQUEST"),
            id: 40,
            dev_status: Some(DevStatus::Wip(Wip {
                description: String::from(""),
                since: None,
            })),
            description: None,
            fields: vec![Field {
                name: String::from("target_system"),
                r#type: String::from("uint8_t"),
                print_format: None,
                r#enum: None,
                display: None,
                units: None,
                increment: None,
                min_value: None,
                max_value: None,
                multiplier: None,
                default: None,
                instance: None,
                invalid: None,
                description: String::from("System ID"),
            }],
            extension_fields: vec![],
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_mavlink_all_fields() {
        let raw = r#"
        <mavlink>
            <include>common.xml</include>
            <include>hello.xml</include>
            <include>all.xml</include>
            <version>1</version>
            <dialect>2</dialect>
            <enums>
                <enum name="UALBERTA_AUTOPILOT_MODE">
                    <description>Available autopilot modes for ualberta uav</description>
                    <entry value="1" name="MODE_MANUAL_DIRECT">
                        <description>Raw input pulse widts sent to output</description>
                    </entry>
                    <entry value="2" name="MODE_MANUAL_SCALED">
                        <description>Inputs are normalized using calibration, the converted back to raw pulse widths for output</description>
                    </entry>
                    <entry value="3" name="MODE_AUTO_PID_ATT"/>
                    <entry value="4" name="MODE_AUTO_PID_VEL"/>
                    <entry value="5" name="MODE_AUTO_PID_POS"/>
                </enum>
            </enums>
            <messages>
                <message id="220" name="NAV_FILTER_BIAS">
                    <description>Accelerometer and Gyro biases from the navigation filter</description>
                    <field type="uint64_t" name="usec">Timestamp (microseconds)</field>
                    <field type="float" name="accel_0">b_f[0]</field>
                    <field type="float" name="accel_1">b_f[1]</field>
                    <field type="float" name="accel_2">b_f[2]</field>
                    <field type="float" name="gyro_0">b_f[0]</field>
                    <field type="float" name="gyro_1">b_f[1]</field>
                    <field type="float" name="gyro_2">b_f[2]</field>
                </message>
            </messages>
        </mavlink>
        "#;

        let parsed = Mavlink::from_str(raw).unwrap();
        let expected = Mavlink {
            include: vec![
                String::from("common.xml"),
                String::from("hello.xml"),
                String::from("all.xml"),
            ],
            version: Some(1),
            dialect: Some(2),
            enums: Some(
                Enums(
                    vec![
                        Enum {
                            name: String::from("UALBERTA_AUTOPILOT_MODE"),
                            bitmask: None,
                            description: Some(
                                Description(
                                    String::from("Available autopilot modes for ualberta uav"),
                                ),
                            ),
                            dev_status: None,
                            entries: vec![
                                Entry {
                                    name: String::from("MODE_MANUAL_DIRECT"),
                                    value: Some(
                                        String::from("1"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: Some(
                                        Description(
                                            String::from("Raw input pulse widts sent to output"),
                                        ),
                                    ),
                                    params: vec![],
                                    dev_status: None,
                                },
                                Entry {
                                    name: String::from("MODE_MANUAL_SCALED"),
                                    value: Some(
                                        String::from("2"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: Some(
                                        Description(
                                            String::from("Inputs are normalized using calibration, the converted back to raw pulse widths for output"),
                                        ),
                                    ),
                                    params: vec![],
                                    dev_status: None,
                                },
                                Entry {
                                    name: String::from("MODE_AUTO_PID_ATT"),
                                    value: Some(
                                        String::from("3"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: None,
                                    params: vec![],
                                    dev_status: None,
                                },
                                Entry {
                                    name: String::from("MODE_AUTO_PID_VEL"),
                                    value: Some(
                                        String::from("4"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: None,
                                    params: vec![],
                                    dev_status: None,
                                },
                                Entry {
                                    name: String::from("MODE_AUTO_PID_POS"),
                                    value: Some(
                                        String::from("5"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: None,
                                    params: vec![],
                                    dev_status: None,
                                },
                            ],
                        },
                    ],
                ),
            ),
            messages: Some(
                Messages(
                    vec![
                        Message {
                            name: String::from("NAV_FILTER_BIAS"),
                            id: 220,
                            dev_status: None,
                            description: Some(
                                Description(
                                    String::from("Accelerometer and Gyro biases from the navigation filter"),
                                ),
                            ),
                            fields: vec![
                                Field {
                                    name: String::from("usec"),
                                    r#type: String::from("uint64_t"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("Timestamp (microseconds)"),
                                },
                                Field {
                                    name: String::from("accel_0"),
                                    r#type: String::from("float"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("b_f[0]"),
                                },
                                Field {
                                    name: String::from("accel_1"),
                                    r#type: String::from("float"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("b_f[1]"),
                                },
                                Field {
                                    name: String::from("accel_2"),
                                    r#type: String::from("float"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("b_f[2]"),
                                },
                                Field {
                                    name: String::from("gyro_0"),
                                    r#type: String::from("float"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("b_f[0]"),
                                },
                                Field {
                                    name: String::from("gyro_1"),
                                    r#type: String::from("float"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("b_f[1]"),
                                },
                                Field {
                                    name: String::from("gyro_2"),
                                    r#type: String::from("float"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("b_f[2]"),
                                },
                            ],
                            extension_fields: vec![],
                        },
                    ],
                ),
            ),
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_mavlink_smoke() {
        let raw = r#"
        <mavlink>
        <include>common.xml</include>
        <enums>
            <enum name="UALBERTA_AUTOPILOT_MODE">
                <description>Available autopilot modes for ualberta uav</description>
                <entry value="1" name="MODE_MANUAL_DIRECT">
                    <description>Raw input pulse widts sent to output</description>
                </entry>
                <entry value="2" name="MODE_MANUAL_SCALED">
                    <description>Inputs are normalized using calibration, the converted back to raw pulse widths for output</description>
                </entry>
                <entry value="3" name="MODE_AUTO_PID_ATT"/>
                <entry value="4" name="MODE_AUTO_PID_VEL"/>
                <entry value="5" name="MODE_AUTO_PID_POS"/>
            </enum>
            <enum name="UALBERTA_NAV_MODE">
                <description>Navigation filter mode</description>
                <entry value="1" name="NAV_AHRS_INIT"/>
                <entry value="2" name="NAV_AHRS">
                    <description>AHRS mode</description>
                </entry>
                <entry value="3" name="NAV_INS_GPS_INIT">
                    <description>INS/GPS initialization mode</description>
                </entry>
                <entry value="4" name="NAV_INS_GPS">
                    <description>INS/GPS mode</description>
                </entry>
            </enum>
            <enum name="UALBERTA_PILOT_MODE">
                <description>Mode currently commanded by pilot</description>
                <entry value="1" name="PILOT_MANUAL"/>
                <entry value="2" name="PILOT_AUTO"/>
                <entry value="3" name="PILOT_ROTO">
                    <description> Rotomotion mode </description>
                </entry>
            </enum>
        </enums>
        <messages>
            <message id="220" name="NAV_FILTER_BIAS">
                <description>Accelerometer and Gyro biases from the navigation filter</description>
                <field type="uint64_t" name="usec">Timestamp (microseconds)</field>
                <field type="float" name="accel_0">b_f[0]</field>
                <field type="float" name="accel_1">b_f[1]</field>
                <field type="float" name="accel_2">b_f[2]</field>
                <field type="float" name="gyro_0">b_f[0]</field>
                <field type="float" name="gyro_1">b_f[1]</field>
                <field type="float" name="gyro_2">b_f[2]</field>
            </message>
            <message id="221" name="RADIO_CALIBRATION">
                <description>Complete set of calibration parameters for the radio</description>
                <field type="uint16_t[3]" name="aileron">Aileron setpoints: left, center, right</field>
                <field type="uint16_t[3]" name="elevator">Elevator setpoints: nose down, center, nose up</field>
                <field type="uint16_t[3]" name="rudder">Rudder setpoints: nose left, center, nose right</field>
                <field type="uint16_t[2]" name="gyro">Tail gyro mode/gain setpoints: heading hold, rate mode</field>
                <field type="uint16_t[5]" name="pitch">Pitch curve setpoints (every 25%)</field>
                <field type="uint16_t[5]" name="throttle">Throttle curve setpoints (every 25%)</field>
            </message>
            <message id="222" name="UALBERTA_SYS_STATUS">
                <description>System status specific to ualberta uav</description>
                <field type="uint8_t" name="mode">System mode, see UALBERTA_AUTOPILOT_MODE ENUM</field>
                <field type="uint8_t" name="nav_mode">Navigation mode, see UALBERTA_NAV_MODE ENUM</field>
                <field type="uint8_t" name="pilot">Pilot mode, see UALBERTA_PILOT_MODE</field>
            </message>
        </messages>
        </mavlink>
        "#;

        let parsed = Mavlink::from_str(raw).unwrap();

        let expected = Mavlink {
            include: vec![
                String::from("common.xml"),
            ],
            version: None,
            dialect: None,
            enums: Some(
                Enums(
                    vec![
                        Enum {
                            name: String::from("UALBERTA_AUTOPILOT_MODE"),
                            bitmask: None,
                            description: Some(
                                Description(
                                    String::from("Available autopilot modes for ualberta uav"),
                                ),
                            ),
                            dev_status: None,
                            entries: vec![
                                Entry {
                                    name: String::from("MODE_MANUAL_DIRECT"),
                                    value: Some(
                                        String::from("1"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: Some(
                                        Description(
                                            String::from("Raw input pulse widts sent to output"),
                                        ),
                                    ),
                                    params: vec![],
                                    dev_status: None,
                                },
                                Entry {
                                    name: String::from("MODE_MANUAL_SCALED"),
                                    value: Some(
                                        String::from("2"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: Some(
                                        Description(
                                            String::from("Inputs are normalized using calibration, the converted back to raw pulse widths for output"),
                                        ),
                                    ),
                                    params: vec![],
                                    dev_status: None,
                                },
                                Entry {
                                    name: String::from("MODE_AUTO_PID_ATT"),
                                    value: Some(
                                        String::from("3"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: None,
                                    params: vec![],
                                    dev_status: None,
                                },
                                Entry {
                                    name: String::from("MODE_AUTO_PID_VEL"),
                                    value: Some(
                                        String::from("4"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: None,
                                    params: vec![],
                                    dev_status: None,
                                },
                                Entry {
                                    name: String::from("MODE_AUTO_PID_POS"),
                                    value: Some(
                                        String::from("5"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: None,
                                    params: vec![],
                                    dev_status: None,
                                },
                            ],
                        },
                        Enum {
                            name: String::from("UALBERTA_NAV_MODE"),
                            bitmask: None,
                            description: Some(
                                Description(
                                    String::from("Navigation filter mode"),
                                ),
                            ),
                            dev_status: None,
                            entries: vec![
                                Entry {
                                    name: String::from("NAV_AHRS_INIT"),
                                    value: Some(
                                        String::from("1"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: None,
                                    params: vec![],
                                    dev_status: None,
                                },
                                Entry {
                                    name: String::from("NAV_AHRS"),
                                    value: Some(
                                        String::from("2"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: Some(
                                        Description(
                                            String::from("AHRS mode"),
                                        ),
                                    ),
                                    params: vec![],
                                    dev_status: None,
                                },
                                Entry {
                                    name: String::from("NAV_INS_GPS_INIT"),
                                    value: Some(
                                        String::from("3"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: Some(
                                        Description(
                                            String::from("INS/GPS initialization mode"),
                                        ),
                                    ),
                                    params: vec![],
                                    dev_status: None,
                                },
                                Entry {
                                    name: String::from("NAV_INS_GPS"),
                                    value: Some(
                                        String::from("4"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: Some(
                                        Description(
                                            String::from("INS/GPS mode"),
                                        ),
                                    ),
                                    params: vec![],
                                    dev_status: None,
                                },
                            ],
                        },
                        Enum {
                            name: String::from("UALBERTA_PILOT_MODE"),
                            bitmask: None,
                            description: Some(
                                Description(
                                    String::from("Mode currently commanded by pilot"),
                                ),
                            ),
                            dev_status: None,
                            entries: vec![
                                Entry {
                                    name: String::from("PILOT_MANUAL"),
                                    value: Some(
                                        String::from("1"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: None,
                                    params: vec![],
                                    dev_status: None,
                                },
                                Entry {
                                    name: String::from("PILOT_AUTO"),
                                    value: Some(
                                        String::from("2"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: None,
                                    params: vec![],
                                    dev_status: None,
                                },
                                Entry {
                                    name: String::from("PILOT_ROTO"),
                                    value: Some(
                                        String::from("3"),
                                    ),
                                    has_location: None,
                                    is_destination: None,
                                    mission_only: None,
                                    description: Some(
                                        Description(
                                            String::from(" Rotomotion mode "),
                                        ),
                                    ),
                                    params: vec![],
                                    dev_status: None,
                                },
                            ],
                        },
                    ],
                ),
            ),
            messages: Some(
                Messages(
                    vec![
                        Message {
                            name: String::from("NAV_FILTER_BIAS"),
                            id: 220,
                            dev_status: None,
                            description: Some(
                                Description(
                                    String::from("Accelerometer and Gyro biases from the navigation filter"),
                                ),
                            ),
                            fields: vec![
                                Field {
                                    name: String::from("usec"),
                                    r#type: String::from("uint64_t"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("Timestamp (microseconds)"),
                                },
                                Field {
                                    name: String::from("accel_0"),
                                    r#type: String::from("float"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("b_f[0]"),
                                },
                                Field {
                                    name: String::from("accel_1"),
                                    r#type: String::from("float"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("b_f[1]"),
                                },
                                Field {
                                    name: String::from("accel_2"),
                                    r#type: String::from("float"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("b_f[2]"),
                                },
                                Field {
                                    name: String::from("gyro_0"),
                                    r#type: String::from("float"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("b_f[0]"),
                                },
                                Field {
                                    name: String::from("gyro_1"),
                                    r#type: String::from("float"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("b_f[1]"),
                                },
                                Field {
                                    name: String::from("gyro_2"),
                                    r#type: String::from("float"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("b_f[2]"),
                                },
                            ],
                            extension_fields: vec![],
                        },
                        Message {
                            name: String::from("RADIO_CALIBRATION"),
                            id: 221,
                            dev_status: None,
                            description: Some(
                                Description(
                                    String::from("Complete set of calibration parameters for the radio"),
                                ),
                            ),
                            fields: vec![
                                Field {
                                    name: String::from("aileron"),
                                    r#type: String::from("uint16_t[3]"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("Aileron setpoints: left, center, right"),
                                },
                                Field {
                                    name: String::from("elevator"),
                                    r#type: String::from("uint16_t[3]"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("Elevator setpoints: nose down, center, nose up"),
                                },
                                Field {
                                    name: String::from("rudder"),
                                    r#type: String::from("uint16_t[3]"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("Rudder setpoints: nose left, center, nose right"),
                                },
                                Field {
                                    name: String::from("gyro"),
                                    r#type: String::from("uint16_t[2]"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("Tail gyro mode/gain setpoints: heading hold, rate mode"),
                                },
                                Field {
                                    name: String::from("pitch"),
                                    r#type: String::from("uint16_t[5]"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("Pitch curve setpoints (every 25%)"),
                                },
                                Field {
                                    name: String::from("throttle"),
                                    r#type: String::from("uint16_t[5]"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("Throttle curve setpoints (every 25%)"),
                                },
                            ],
                            extension_fields: vec![],
                        },
                        Message {
                            name: String::from("UALBERTA_SYS_STATUS"),
                            id: 222,
                            dev_status: None,
                            description: Some(
                                Description(
                                    String::from("System status specific to ualberta uav"),
                                ),
                            ),
                            fields: vec![
                                Field {
                                    name: String::from("mode"),
                                    r#type: String::from("uint8_t"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("System mode, see UALBERTA_AUTOPILOT_MODE ENUM"),
                                },
                                Field {
                                    name: String::from("nav_mode"),
                                    r#type: String::from("uint8_t"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("Navigation mode, see UALBERTA_NAV_MODE ENUM"),
                                },
                                Field {
                                    name: String::from("pilot"),
                                    r#type: String::from("uint8_t"),
                                    print_format: None,
                                    r#enum: None,
                                    display: None,
                                    units: None,
                                    increment: None,
                                    min_value: None,
                                    max_value: None,
                                    multiplier: None,
                                    default: None,
                                    instance: None,
                                    invalid: None,
                                    description: String::from("Pilot mode, see UALBERTA_PILOT_MODE"),
                                },
                            ],
                            extension_fields: vec![],
                        },
                    ],
                ),
            ),
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_mavlink_optional_fields() {
        // all fields are optinal :)
        let raw = r#"
        <mavlink>
        </mavlink>
        "#;

        let parsed = Mavlink::from_str(raw).unwrap();

        let expected = Mavlink {
            include: vec![],
            version: None,
            dialect: None,
            enums: None,
            messages: None,
        };

        assert_eq!(parsed, expected);
    }
}
