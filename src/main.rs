use windows::{
  core::{GUID,HSTRING,PCWSTR,PWSTR},
  Win32::{
    Foundation::{ERROR_SUCCESS,HANDLE,INVALID_HANDLE_VALUE,WIN32_ERROR},
    NetworkManagement::WiFi::{
      WlanCloseHandle, WlanEnumInterfaces, WlanFreeMemory, WlanGetProfile, WlanGetProfileList,
      WlanOpenHandle,WLAN_API_VERSION_2_0, WLAN_INTERFACE_INFO_LIST,
      WLAN_PROFILE_GET_PLAINTEXT_KEY, WLAN_PROFILE_INFO_LIST,
    },
  },
};

fn main() {
    println!("Hello, world!");
}
