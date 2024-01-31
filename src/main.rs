use std::{ffi::OsString, os::windows::ffi::OsStringExt};

use windows::{
    core::{GUID, HSTRING, PCWSTR, PWSTR},
    Data::Xml::Dom::{XmlDocument, XmlElement},
    Win32::{
        Foundation::{HANDLE, INVALID_HANDLE_VALUE, WIN32_ERROR},
        NetworkManagement::WiFi::{
            WlanCloseHandle, WlanEnumInterfaces, WlanFreeMemory, WlanGetProfile, WlanGetProfileList,
            WlanOpenHandle, WLAN_API_VERSION_2_0, WLAN_INTERFACE_INFO_LIST, WLAN_PROFILE_GET_PLAINTEXT_KEY,
            WLAN_PROFILE_INFO_LIST,
        },
    },
};

// Function to open a WLAN handle
fn open_wlan_handle(api_version: u32) -> Result<HANDLE, windows::core::Error> {
    let mut negotiatied_version: u32 = 0;
    let mut wlan_handle: HANDLE = INVALID_HANDLE_VALUE;

    let result: u32 =
        unsafe { WlanOpenHandle(api_version, None, &mut negotiatied_version, &mut wlan_handle) };

    WIN32_ERROR(result).ok()?;

    Ok(wlan_handle)
}

// Function to enumerate WLAN interfaces
fn enum_wlan_interfaces(handle: HANDLE) -> Result<*mut WLAN_INTERFACE_INFO_LIST, windows::core::Error> {
    let mut interface_ptr: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();

    let result: u32 = unsafe { WlanEnumInterfaces(handle, None, &mut interface_ptr) };

    WIN32_ERROR(result).ok()?;

    Ok(interface_ptr)
}

// Function to retrieve WLAN profiles for a specific interface
fn grab_interface_profiles(handle: HANDLE,interface_guid: &GUID,) -> Result<*const WLAN_PROFILE_INFO_LIST, windows::core::Error> {
    let mut wlan_profiles_ptr = std::ptr::null_mut();

    let result = unsafe { WlanGetProfileList(handle, interface_guid, None, &mut wlan_profiles_ptr) };

    WIN32_ERROR(result).ok()?;

    Ok(wlan_profiles_ptr)
}

// Function to parse a UTF-16 slice into an OsString
fn parse_utf16_slice(string_slice: &[u16]) -> Option<OsString> {
    let null_index: usize = string_slice.iter().position(|c: &u16| c == &0)?;
    Some(OsString::from_wide(&string_slice[..null_index]))
}

// Function to load XML data into an XmlDocument
fn load_xml_data(xml: &OsString) -> Result<XmlDocument, windows::core::Error> {
    let xml_document: XmlDocument = XmlDocument::new()?;
    xml_document.LoadXml(&HSTRING::from(xml))?;
    Ok(xml_document)
}

// Function to traverse the XML tree and extract data based on node path
fn traverse_xml_tree(xml: &XmlElement, node_path: &[&str]) -> Option<String> {
    let mut subtree_list: windows::Data::Xml::Dom::XmlNodeList = xml.ChildNodes().ok()?;
    let last_node_name: &&str = node_path.last()?;

    'node_traverse: for node in node_path {
        let node_name: OsString = OsString::from_wide(&node.encode_utf16().collect::<Vec<u16>>());

        for subtree_value in &subtree_list {
            let element_name: HSTRING = match subtree_value.NodeName() {
                Ok(name) => name,
                Err(_) => continue,
            };
            if element_name.to_os_string() == node_name {
                if element_name.to_os_string().to_string_lossy().to_string() == last_node_name.to_string() {
                    return Some(subtree_value.InnerText().ok()?.to_string());
                }
                subtree_list = subtree_value.ChildNodes().ok()?;
                continue 'node_traverse;
            }
        }
    }
    None
}

// Function to get the XML data of a specific WLAN profile
fn get_profile_xml(
    handle: HANDLE,
    interface_guid: &GUID,
    profile_name: &OsString,
) -> Result<OsString, windows::core::Error> {
    let mut profile_xml_data: PWSTR = PWSTR::null();
    let mut profile_get_flags: u32 = WLAN_PROFILE_GET_PLAINTEXT_KEY;

    let result: u32 = unsafe {
        WlanGetProfile(
            handle,
            interface_guid,
            PCWSTR(HSTRING::from(profile_name).as_ptr()),
            None,
            &mut profile_xml_data,
            Some(&mut profile_get_flags),
            None,
        )
    };

    WIN32_ERROR(result).ok()?;

    let xml_string: HSTRING = match unsafe { profile_xml_data.to_hstring() } {
        Ok(data) => data,
        Err(e) => {
            unsafe {
                WlanFreeMemory(profile_xml_data.as_ptr().cast());
            }
            return Err(e);
        }
    };
    Ok(xml_string.to_os_string())
}

fn main() {
    // Opening the WLAN handle
    let wlan_handle: HANDLE = open_wlan_handle(WLAN_API_VERSION_2_0).expect("Failed to open WLAN handle!");

    // Enumerating WLAN interfaces
    let interface_ptr: *mut WLAN_INTERFACE_INFO_LIST = match enum_wlan_interfaces(wlan_handle) {
        Ok(interfaces) => interfaces,
        Err(e) => {
            eprintln!("Failed to enumerate interfaces: {}", e);
            unsafe {
                WlanCloseHandle(wlan_handle, None);
            }
            std::process::exit(1);
        }
    };

    // Extracting interface information from the pointer
    let interfaces_list = unsafe {
        std::slice::from_raw_parts(
            (*interface_ptr).InterfaceInfo.as_ptr(),
            (*interface_ptr).dwNumberOfItems as usize,
        )
    };

    // Iterating through each WLAN interface
    for interface_info in interfaces_list {
        // Parsing the UTF-16 slice for interface description
        let _interface_description: OsString =
            match parse_utf16_slice(interface_info.strInterfaceDescription.as_slice()) {
                Some(name) => name,
                None => {
                    eprintln!("Could not parse our interface description");
                    continue;
                }
            };

        // Retrieving WLAN profiles for the interface
        let wlan_profile_ptr: *const WLAN_PROFILE_INFO_LIST =
            match grab_interface_profiles(wlan_handle, &interface_info.InterfaceGuid) {
                Ok(profiles) => profiles,
                Err(_e) => {
                    eprintln!("Failed to retrieve profiles");
                    continue;
                }
            };

        // Extracting WLAN profile information from the pointer
        let wlan_profile_list: &[windows::Win32::NetworkManagement::WiFi::WLAN_PROFILE_INFO] = unsafe {
            std::slice::from_raw_parts((*wlan_profile_ptr).ProfileInfo.as_ptr(), (*wlan_profile_ptr).dwNumberOfItems as usize)
        };

        // Iterating through each WLAN profile
        for profile in wlan_profile_list {
            // Parsing the UTF-16 slice for profile name
            let profile_name = match parse_utf16_slice(&profile.strProfileName) {
                Some(name) => name,
                None => {
                    eprintln!("Could not parse profile name");
                    continue;
                }
            };

            // Retrieving XML data for the profile
            let profile_xml_data: OsString = match get_profile_xml(wlan_handle, &interface_info.InterfaceGuid, &profile_name) {
                Ok(data) => data,
                Err(_e) => {
                    eprintln!("Failed to retrieve profile XML");
                    continue;
                }
            };

            // Loading XML data into an XmlDocument
            let xml_document: XmlDocument = match load_xml_data(&profile_xml_data) {
                Ok(xml) => xml,
                Err(_e) => {
                    eprintln!("Failed to extract XML document");
                    continue;
                }
            };

            // Getting the root element of the XML document
            let root: XmlElement = match xml_document.DocumentElement() {
                Ok(root) => root,
                Err(_e) => {
                    eprintln!("Failed to get document root for profile XML");
                    continue;
                }
            };

            // Traversing the XML tree to extract authentication type
            let auth_type: String = match traverse_xml_tree(&root, &["MSM", "security", "authEncryption", "authentication"]) {
                Some(t) => t,
                None => {
                    eprintln!("Failed to get the auth type for this profile");
                    continue;
                }
            };

            // Printing information based on authentication type
            match auth_type.as_str() {
                "open" => {
                    println!("Wi-Fi Name: {}, No password", profile_name.to_string_lossy().to_string());
                },
                "WPA2" | "WPA2PSK" => {
                    if let Some(password) = traverse_xml_tree(&root, &["MSM", "security", "sharedKey", "keyMaterial"]) {
                        println!("Wi-Fi Name: {}, Authentication: {} Password: {}", profile_name.to_string_lossy().to_string(), auth_type, password);
                    }
                },
                _ => {
                    println!("Wi-Fi Name: {}, Authentication: {}", profile_name.to_string_lossy().to_string(), auth_type);
                }
            }
        }
    }

    // Freeing memory for WLAN interface information
    unsafe {
        WlanFreeMemory(interface_ptr.cast());
    }

    // Closing the WLAN handle
    unsafe {
        WlanCloseHandle(wlan_handle, None);
    }
}
