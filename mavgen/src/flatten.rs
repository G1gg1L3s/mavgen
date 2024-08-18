use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use crate::{
    parser::{self, MavlinkFile},
    xml,
};

#[derive(Debug, Clone, PartialEq)]
pub struct MavlinkModule {
    pub path: PathBuf,
    pub version: Option<u8>,
    pub dialect: Option<u8>,
    pub enums: Vec<xml::Enum>,
    pub messages: Vec<xml::Message>,
}

#[derive(Debug, Default)]
struct MessageAndEnumCollector<'a> {
    messages: Vec<xml::Message>,
    enums: Vec<xml::Enum>,
    /// Map from enum name to enum index in the self.enums.
    ///
    /// Used to preserve the enum order but speed up search of the same enums
    /// to merge.
    enum_index: HashMap<&'a str, usize>,

    processed: HashSet<&'a Path>,
}

fn flatten_recursive<'a>(
    collector: &mut MessageAndEnumCollector<'a>,
    files: &'a HashMap<PathBuf, parser::MavlinkFile>,
    module: &'a MavlinkFile,
) {
    for include in &module.normalised_includes {
        let file = files
            .get(include)
            .expect("bug: the file should already be parsed");

        if !collector.processed.contains(include.as_path()) {
            collector.processed.insert(include);
            flatten_recursive(collector, files, file);
        }
    }

    if let Some(messages) = &module.mavlink.messages {
        collector.messages.extend_from_slice(&messages.0);
    }

    if let Some(enums) = &module.mavlink.enums {
        collector.enums.reserve(enums.0.len());
        collector.enum_index.reserve(enums.0.len());

        for enum_ in &enums.0 {
            if let Some(idx) = collector.enum_index.get(enum_.name.as_str()) {
                let target_enum = &mut collector.enums[*idx];
                target_enum.entries.extend_from_slice(&enum_.entries);
            } else {
                let idx = collector.enums.len();
                collector.enums.push(enum_.clone());
                collector.enum_index.insert(&enum_.name, idx);
            }
        }
    }
}

pub fn flatten(
    files: &HashMap<PathBuf, parser::MavlinkFile>,
    normalised: &Path,
) -> std::io::Result<MavlinkModule> {
    let module = files
        .get(normalised)
        .expect("bug: the file should be parsed");

    let mut collector = MessageAndEnumCollector::default();
    flatten_recursive(&mut collector, files, module);

    Ok(MavlinkModule {
        path: normalised.to_owned(),
        version: module.mavlink.version,
        dialect: module.mavlink.dialect,
        enums: collector.enums,
        messages: collector.messages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hard_xml::XmlRead;
    use parser::{tests::MockWorld, Parser};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_flatten_one() {
        let mavlink = xml::Mavlink {
            include: vec![],
            version: Some(1),
            dialect: Some(2),
            enums: Some(xml::Enums(vec![xml::Enum {
                name: "SOME_ENUM".into(),
                bitmask: None,
                description: None,
                dev_status: None,
                entries: vec![
                    xml::Entry::new_min("SOME_ENUM_1", Some("1")),
                    xml::Entry::new_min("SOME_ENUM_2", Some("2")),
                    xml::Entry::new_min("SOME_ENUM_3", Some("3")),
                ],
            }])),
            messages: Some(xml::Messages(vec![xml::Message {
                name: "SOME_MESSAGE".into(),
                id: 12345,
                dev_status: None,
                description: None,
                fields: vec![
                    xml::Field::new_min("some_field_1", "uint8_t"),
                    xml::Field::new_min("some_field_2", "uint8_t"),
                    xml::Field::new_min("some_field_3", "uint8_t"),
                ],
                extension_fields: vec![],
            }])),
        };

        let files = HashMap::from([(
            PathBuf::from("/cwd/test.xml"),
            parser::MavlinkFile {
                mavlink: mavlink.clone(),
                normalised_includes: vec![],
            },
        )]);

        let module = flatten(&files, Path::new("/cwd/test.xml")).unwrap();
        assert_eq!(module.path, Path::new("/cwd/test.xml"));
        assert_eq!(module.version, Some(1));
        assert_eq!(module.dialect, Some(2));
        assert_eq!(module.enums, mavlink.enums.as_ref().unwrap().0);
        assert_eq!(module.messages, mavlink.messages.as_ref().unwrap().0);
    }

    #[test]
    fn test_chain() {
        let world = MockWorld(HashMap::from_iter([
            (
                PathBuf::from("/cwd/test-1.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <include>test-2.xml</include>
                        <dialect>1</dialect>
                        <version>2</version>
                        <enums>
                            <enum name="ICAROUS_TRACK_BAND_TYPES">
                                <entry name="ICAROUS_TRACK_BAND_TYPE_NONE" value="0"/>
                                <entry name="ICAROUS_TRACK_BAND_TYPE_NEAR" value="1"/>
                                <entry name="ICAROUS_TRACK_BAND_TYPE_RECOVERY" value="2"/>
                            </enum>
                            <enum name="MAV_CMD">
                                <entry value="218" name="MAV_CMD_DO_AUX_FUNCTION"/>
                            </enum>
                        </enums>
                        <messages>
                            <message id="151" name="SET_MAG_OFFSETS">
                                <deprecated since="2014-07" replaced_by="MAV_CMD_PREFLIGHT_SET_SENSOR_OFFSETS"/>
                                <description>Set the magnetometer offsets</description>
                                <field type="uint8_t" name="target_system">System ID.</field>
                                <field type="uint8_t" name="target_component">Component ID.</field>
                                <field type="int16_t" name="mag_ofs_x">Magnetometer X offset.</field>
                                <field type="int16_t" name="mag_ofs_y">Magnetometer Y offset.</field>
                                <field type="int16_t" name="mag_ofs_z">Magnetometer Z offset.</field>
                            </message>
                        </messages>
                    </mavlink>
                "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-2.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <dialect>3</dialect>
                        <version>4</version>
                        <include>test-3.xml</include>
                        <enums>
                            <enum name="MAV_CMD">
                                <entry value="5001" name="MAV_CMD_NAV_FENCE_POLYGON_VERTEX_INCLUSION"/>
                            </enum>
                        </enums>
                        <messages>
                            <message id="2" name="SYSTEM_TIME">
                                <field type="uint64_t" name="time_unix_usec" units="us"/>
                                <field type="uint32_t" name="time_boot_ms" units="ms"/>
                            </message>
                        </messages>
                    </mavlink>
                "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-3.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <dialect>4</dialect>
                        <version>5</version>
                        <enums>
                            <enum name="MAV_CMD">
                                <entry name="MAV_CMD_RESET_MPPT" value="40001"/>
                                <entry name="MAV_CMD_PAYLOAD_CONTROL" value="40002"/>
                            </enum>
                            <enum name="GSM_MODEM_TYPE">
                                <entry value="0" name="GSM_MODEM_TYPE_UNKNOWN"/>
                                <entry value="1" name="GSM_MODEM_TYPE_HUAWEI_E3372"/>
                            </enum>
                        </enums>
                        <messages>
                            <message id="223" name="COMMAND_INT_STAMPED">
                                <field type="uint32_t" name="utc_time"/>
                                <field type="uint64_t" name="vehicle_timestamp"/>
                            </message>
                        </messages>
                    </mavlink>
                    "#,
                ),
            ),
        ]));

        let mut parser = Parser::new(world);
        parser.parse(Path::new("test-1.xml"));
        let files = parser.finish().unwrap();

        let expected = xml::Mavlink::from_str(
            r#"<?xml version="1.0"?>
            <mavlink>
                <dialect>1</dialect>
                <version>2</version>
                <enums>
                    <enum name="MAV_CMD">
                        <entry name="MAV_CMD_RESET_MPPT" value="40001"/>
                        <entry name="MAV_CMD_PAYLOAD_CONTROL" value="40002"/>
                        <entry value="5001" name="MAV_CMD_NAV_FENCE_POLYGON_VERTEX_INCLUSION"/>
                        <entry value="218" name="MAV_CMD_DO_AUX_FUNCTION"/>
                    </enum>
                    <enum name="GSM_MODEM_TYPE">
                        <entry value="0" name="GSM_MODEM_TYPE_UNKNOWN"/>
                        <entry value="1" name="GSM_MODEM_TYPE_HUAWEI_E3372"/>
                    </enum>
                    <enum name="ICAROUS_TRACK_BAND_TYPES">
                        <entry name="ICAROUS_TRACK_BAND_TYPE_NONE" value="0"/>
                        <entry name="ICAROUS_TRACK_BAND_TYPE_NEAR" value="1"/>
                        <entry name="ICAROUS_TRACK_BAND_TYPE_RECOVERY" value="2"/>
                    </enum>
                </enums>
                <messages>
                    <message id="223" name="COMMAND_INT_STAMPED">
                        <field type="uint32_t" name="utc_time"/>
                        <field type="uint64_t" name="vehicle_timestamp"/>
                    </message>
                    <message id="2" name="SYSTEM_TIME">
                        <field type="uint64_t" name="time_unix_usec" units="us"/>
                        <field type="uint32_t" name="time_boot_ms" units="ms"/>
                    </message>
                    <message id="151" name="SET_MAG_OFFSETS">
                        <deprecated since="2014-07" replaced_by="MAV_CMD_PREFLIGHT_SET_SENSOR_OFFSETS"/>
                        <description>Set the magnetometer offsets</description>
                        <field type="uint8_t" name="target_system">System ID.</field>
                        <field type="uint8_t" name="target_component">Component ID.</field>
                        <field type="int16_t" name="mag_ofs_x">Magnetometer X offset.</field>
                        <field type="int16_t" name="mag_ofs_y">Magnetometer Y offset.</field>
                        <field type="int16_t" name="mag_ofs_z">Magnetometer Z offset.</field>
                    </message>
                </messages>
            </mavlink>
            "#,
        ).unwrap();

        let module = flatten(&files, Path::new("/cwd/test-1.xml")).unwrap();

        assert_eq!(module.path, Path::new("/cwd/test-1.xml"));
        assert_eq!(module.dialect, expected.dialect);
        assert_eq!(module.version, expected.version);
        assert_eq!(module.messages, expected.messages.unwrap().0);
        assert_eq!(module.enums, expected.enums.unwrap().0);
    }

    #[test]
    fn test_diamond() {
        let world = MockWorld(HashMap::from_iter([
            (
                PathBuf::from("/cwd/test-1.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <include>test-common.xml</include>
                        <include>test-2.xml</include>
                        <dialect>1</dialect>
                        <version>2</version>
                        <enums>
                            <enum name="ICAROUS_TRACK_BAND_TYPES">
                                <entry name="ICAROUS_TRACK_BAND_TYPE_NONE" value="0"/>
                                <entry name="ICAROUS_TRACK_BAND_TYPE_NEAR" value="1"/>
                                <entry name="ICAROUS_TRACK_BAND_TYPE_RECOVERY" value="2"/>
                            </enum>
                            <enum name="MAV_CMD">
                                <entry value="218" name="MAV_CMD_DO_AUX_FUNCTION"/>
                            </enum>
                        </enums>
                        <messages>
                            <message id="151" name="SET_MAG_OFFSETS">
                                <deprecated since="2014-07" replaced_by="MAV_CMD_PREFLIGHT_SET_SENSOR_OFFSETS"/>
                                <description>Set the magnetometer offsets</description>
                                <field type="uint8_t" name="target_system">System ID.</field>
                                <field type="uint8_t" name="target_component">Component ID.</field>
                                <field type="int16_t" name="mag_ofs_x">Magnetometer X offset.</field>
                                <field type="int16_t" name="mag_ofs_y">Magnetometer Y offset.</field>
                                <field type="int16_t" name="mag_ofs_z">Magnetometer Z offset.</field>
                            </message>
                        </messages>
                    </mavlink>
                "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-2.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <dialect>3</dialect>
                        <version>4</version>
                        <include>test-common.xml</include>
                        <include>test-3.xml</include>
                        <enums>
                            <enum name="MAV_CMD">
                                <entry value="5001" name="MAV_CMD_NAV_FENCE_POLYGON_VERTEX_INCLUSION"/>
                            </enum>
                        </enums>
                        <messages>
                            <message id="2" name="SYSTEM_TIME">
                                <field type="uint64_t" name="time_unix_usec" units="us"/>
                                <field type="uint32_t" name="time_boot_ms" units="ms"/>
                            </message>
                        </messages>
                    </mavlink>
                "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-3.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <dialect>4</dialect>
                        <version>5</version>
                        <enums>
                            <enum name="MAV_CMD">
                                <entry name="MAV_CMD_RESET_MPPT" value="40001"/>
                                <entry name="MAV_CMD_PAYLOAD_CONTROL" value="40002"/>
                            </enum>
                            <enum name="GSM_MODEM_TYPE">
                                <entry value="0" name="GSM_MODEM_TYPE_UNKNOWN"/>
                                <entry value="1" name="GSM_MODEM_TYPE_HUAWEI_E3372"/>
                            </enum>
                        </enums>
                        <messages>
                            <message id="223" name="COMMAND_INT_STAMPED">
                                <field type="uint32_t" name="utc_time"/>
                                <field type="uint64_t" name="vehicle_timestamp"/>
                            </message>
                        </messages>
                    </mavlink>
                    "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-common.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <dialect>5</dialect>
                        <version>6</version>
                        <enums>
                            <enum name="MAV_CMD">
                                <entry value="215" name="MAV_CMD_DO_SET_RESUME_REPEAT_DIST"/>
                                <entry value="216" name="MAV_CMD_DO_SPRAYER"/>
                            </enum>
                            <enum name="WIFI_CONFIG_AP_MODE">
                                <description>WiFi Mode.</description>
                                <entry value="0" name="WIFI_CONFIG_AP_MODE_UNDEFINED">
                                    <description>WiFi mode is undefined.</description>
                                </entry>
                                <entry value="1" name="WIFI_CONFIG_AP_MODE_AP">
                                    <description>WiFi configured as an access point.</description>
                                </entry>
                                <entry value="2" name="WIFI_CONFIG_AP_MODE_STATION">
                                    <description>WiFi configured as a station connected to an existing local WiFi network.</description>
                                </entry>
                                <entry value="3" name="WIFI_CONFIG_AP_MODE_DISABLED">
                                    <description>WiFi disabled.</description>
                                </entry>
                            </enum>
                        </enums>
                        <messages>
                            <message id="0" name="HEARTBEAT">
                                <description>The heartbeat message shows that a system or component is present and responding. The type and autopilot fields (along with the message component id), allow the receiving system to treat further messages from this system appropriately (e.g. by laying out the user interface based on the autopilot). This microservice is documented at https://mavlink.io/en/services/heartbeat.html</description>
                                <field type="uint8_t" name="type" enum="MAV_TYPE">Vehicle or component type. For a flight controller component the vehicle type (quadrotor, helicopter, etc.). For other components the component type (e.g. camera, gimbal, etc.). This should be used in preference to component id for identifying the component type.</field>
                                <field type="uint8_t" name="autopilot" enum="MAV_AUTOPILOT">Autopilot type / class. Use MAV_AUTOPILOT_INVALID for components that are not flight controllers.</field>
                                <field type="uint8_t" name="base_mode" enum="MAV_MODE_FLAG" display="bitmask">System mode bitmap.</field>
                                <field type="uint32_t" name="custom_mode">A bitfield for use for autopilot-specific flags</field>
                                <field type="uint8_t" name="system_status" enum="MAV_STATE">System status flag.</field>
                                <field type="uint8_t_mavlink_version" name="mavlink_version">MAVLink version, not writable by user, gets added by protocol because of magic data type: uint8_t_mavlink_version</field>
                            </message>
                        </messages>
                    </mavlink>
                    "#,
                ),
            ),
        ]));

        let mut parser = Parser::new(world);
        parser.parse(Path::new("test-1.xml"));
        let files = parser.finish().unwrap();

        let expected = xml::Mavlink::from_str(
            r#"<?xml version="1.0"?>
            <mavlink>
                <dialect>1</dialect>
                <version>2</version>
                <enums>
                    <enum name="MAV_CMD">
                        <entry value="215" name="MAV_CMD_DO_SET_RESUME_REPEAT_DIST"/>
                        <entry value="216" name="MAV_CMD_DO_SPRAYER"/>
                        <entry name="MAV_CMD_RESET_MPPT" value="40001"/>
                        <entry name="MAV_CMD_PAYLOAD_CONTROL" value="40002"/>
                        <entry value="5001" name="MAV_CMD_NAV_FENCE_POLYGON_VERTEX_INCLUSION"/>
                        <entry value="218" name="MAV_CMD_DO_AUX_FUNCTION"/>
                    </enum>
                    <enum name="WIFI_CONFIG_AP_MODE">
                        <description>WiFi Mode.</description>
                        <entry value="0" name="WIFI_CONFIG_AP_MODE_UNDEFINED">
                            <description>WiFi mode is undefined.</description>
                        </entry>
                        <entry value="1" name="WIFI_CONFIG_AP_MODE_AP">
                            <description>WiFi configured as an access point.</description>
                        </entry>
                        <entry value="2" name="WIFI_CONFIG_AP_MODE_STATION">
                            <description>WiFi configured as a station connected to an existing local WiFi network.</description>
                        </entry>
                        <entry value="3" name="WIFI_CONFIG_AP_MODE_DISABLED">
                            <description>WiFi disabled.</description>
                        </entry>
                    </enum>
                    <enum name="GSM_MODEM_TYPE">
                        <entry value="0" name="GSM_MODEM_TYPE_UNKNOWN"/>
                        <entry value="1" name="GSM_MODEM_TYPE_HUAWEI_E3372"/>
                    </enum>
                    <enum name="ICAROUS_TRACK_BAND_TYPES">
                        <entry name="ICAROUS_TRACK_BAND_TYPE_NONE" value="0"/>
                        <entry name="ICAROUS_TRACK_BAND_TYPE_NEAR" value="1"/>
                        <entry name="ICAROUS_TRACK_BAND_TYPE_RECOVERY" value="2"/>
                    </enum>
                </enums>
                <messages>
                    <message id="0" name="HEARTBEAT">
                        <description>The heartbeat message shows that a system or component is present and responding. The type and autopilot fields (along with the message component id), allow the receiving system to treat further messages from this system appropriately (e.g. by laying out the user interface based on the autopilot). This microservice is documented at https://mavlink.io/en/services/heartbeat.html</description>
                        <field type="uint8_t" name="type" enum="MAV_TYPE">Vehicle or component type. For a flight controller component the vehicle type (quadrotor, helicopter, etc.). For other components the component type (e.g. camera, gimbal, etc.). This should be used in preference to component id for identifying the component type.</field>
                        <field type="uint8_t" name="autopilot" enum="MAV_AUTOPILOT">Autopilot type / class. Use MAV_AUTOPILOT_INVALID for components that are not flight controllers.</field>
                        <field type="uint8_t" name="base_mode" enum="MAV_MODE_FLAG" display="bitmask">System mode bitmap.</field>
                        <field type="uint32_t" name="custom_mode">A bitfield for use for autopilot-specific flags</field>
                        <field type="uint8_t" name="system_status" enum="MAV_STATE">System status flag.</field>
                        <field type="uint8_t_mavlink_version" name="mavlink_version">MAVLink version, not writable by user, gets added by protocol because of magic data type: uint8_t_mavlink_version</field>
                    </message>
                    <message id="223" name="COMMAND_INT_STAMPED">
                        <field type="uint32_t" name="utc_time"/>
                        <field type="uint64_t" name="vehicle_timestamp"/>
                    </message>
                    <message id="2" name="SYSTEM_TIME">
                        <field type="uint64_t" name="time_unix_usec" units="us"/>
                        <field type="uint32_t" name="time_boot_ms" units="ms"/>
                    </message>
                    <message id="151" name="SET_MAG_OFFSETS">
                        <deprecated since="2014-07" replaced_by="MAV_CMD_PREFLIGHT_SET_SENSOR_OFFSETS"/>
                        <description>Set the magnetometer offsets</description>
                        <field type="uint8_t" name="target_system">System ID.</field>
                        <field type="uint8_t" name="target_component">Component ID.</field>
                        <field type="int16_t" name="mag_ofs_x">Magnetometer X offset.</field>
                        <field type="int16_t" name="mag_ofs_y">Magnetometer Y offset.</field>
                        <field type="int16_t" name="mag_ofs_z">Magnetometer Z offset.</field>
                    </message>
                </messages>
            </mavlink>
            "#,
        ).unwrap();

        let module = flatten(&files, Path::new("/cwd/test-1.xml")).unwrap();

        assert_eq!(module.path, Path::new("/cwd/test-1.xml"));
        assert_eq!(module.dialect, expected.dialect);
        assert_eq!(module.version, expected.version);
        assert_eq!(module.messages, expected.messages.unwrap().0);
        assert_eq!(module.enums, expected.enums.unwrap().0);
    }
}
