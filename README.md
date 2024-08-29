# sbchargelimit
It keeps your laptop's battery charge below a certain threshold, using a SwitchBot smart plug.

## Requirements
- A laptop computer with battery and Bluetooth LE (Tested on Windows. Probably also works on Linux)
- SwitchBot Plug Mini smart plug

## Usage
1. Plug your laptop AC adapter through the smart plug and configure it.
1. Launch this program once. It will fail immediately but sample configuration file will be generated at `C:\Users\<UserName>\AppData\Roaming\sbchargelimit\config\default-config.toml`.
1. Write the MAC address of your smart plug like this:
    ```toml
    [plug_mini]
    addr = "01:23:45:67:89:AB"
    ```
1. Launch the program again. Also, you can register it as a startup program.