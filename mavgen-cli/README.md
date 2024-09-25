# mavgen-cli

This crate contains a CLI tool for the mavgen.

```
Usage: mavgen-cli --output <OUTPUT> <INPUT>...

Arguments:
  <INPUT>...  Path to definition files

Options:
  -o, --output <OUTPUT>  Output file or directory
```

The tool can compile directories or separate files. The general rules are as follows:

1. If input is one file and output is a file, the result will be a module:
   ```
   $ mavgen-cli mavlink/message_definitions/v1.0/ardupilotmega.xml -o ardupilot.rs
   ```

2. If input is one file and output is a dir, the result will be a module inside that dir:
   ```
   $ mkdir messages
   $ mavgen-cli mavlink/message_definitions/v1.0/ardupilotmega.xml -o messages/
   $ ls messages
   ardupilotmega.rs
   ```

3. The input can be a list of directories or files, in which case the result will be a directory of modules:
   ```
   $ mkdir messages
   $ mavgen-cli mavlink/message_definitions/v1.0/ -o messages
   $ ls messages/
   all.rs            avssuas.rs      cubepilot.rs    matrixpilot.rs  paparazzi.rs          storm32.rs   u_avionix.rs
   ardupilotmega.rs  common.rs       development.rs  minimal.rs      python_array_test.rs  test.rs
   asluav.rs         cs_air_link.rs  icarous.rs      mod.rs          standard.rs           ualberta.rs
   ```
