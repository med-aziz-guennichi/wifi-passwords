
use std::{ffi::OsString, os::windows::ffi::OsStringExt};

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

fn open_wlan_handle(api_version:u32) -> Result<HANDLE,windows::core::Error> {
  let mut negotiatied_version = 0;
  let mut wlan_handle = INVALID_HANDLE_VALUE;

  let result = unsafe {WlanOpenHandle(api_version, None, &mut negotiatied_version, &mut wlan_handle)};

  WIN32_ERROR(result).ok()?;

  Ok(wlan_handle)
}

fn enum_wlan_interfaces(handle:HANDLE) -> Result<*mut WLAN_INTERFACE_INFO_LIST,windows::core::Error> {
  let mut interface_ptr = std::ptr::null_mut();

  let result = unsafe {WlanEnumInterfaces(handle, None, &mut interface_ptr)};

  WIN32_ERROR(result).ok()?;

  Ok(interface_ptr)
}

fn grab_interface_profiles(handle:HANDLE,interface_guid:&GUID) -> Result<*const WLAN_PROFILE_INFO_LIST, windows::core::Error> {
  let mut wlan_profiles_ptr = std::ptr::null_mut();

  let result = unsafe {WlanGetProfileList(handle, interface_guid, None, &mut wlan_profiles_ptr)};

  WIN32_ERROR(result).ok()?;

  Ok(wlan_profiles_ptr)
}

fn parse_utf16_slice(string_slice:&[u16]) -> Option<OsString> {
  let null_index = string_slice.iter().position(|c| c == &0)?;
  Some(OsString::from_wide(&string_slice[..null_index]))
}

fn main() {
    let wlan_handle = open_wlan_handle(WLAN_API_VERSION_2_0).expect("Failed to open WLAN handle!");

    let interface_ptr = match enum_wlan_interfaces(wlan_handle) {
      Ok(interfaces) => interfaces,
      Err(e) => {
        eprintln!("Failed to enumerate interfaces: {}", e);
        unsafe { WlanCloseHandle(wlan_handle, None) };
        std::process::exit(1);
      }
    };

    let interfaces_list = unsafe {
        std::slice::from_raw_parts((*interface_ptr).InterfaceInfo.as_ptr(),(*interface_ptr).dwNumberOfItems as usize)
    };
    for interface_info in interfaces_list {
      let interface_description = match parse_utf16_slice(interface_info.strInterfaceDescription.as_slice()){
        Some(name) => name,
        None => {
          eprintln!("Could not parse our interface description");
          continue;
        }
      };
      let wlan_profile_ptr = match grab_interface_profiles(wlan_handle, &interface_info.InterfaceGuid) {
        Ok(profiles) => profiles,
        Err(_e) => {
          eprintln!("Failed to retrieve profiles");
          continue;
        }
      };
      let wlan_profile_list = unsafe {std::slice::from_raw_parts((*wlan_profile_ptr).ProfileInfo.as_ptr(), (*wlan_profile_ptr).dwNumberOfItems as usize)};

      for profile in wlan_profile_list {
        let profile_name = match parse_utf16_slice(&profile.strProfileName) {
          Some(name) => name,
          None => {
            eprintln!("Could not parse profile name");
            continue;
          }
        };
      }
    }
}
