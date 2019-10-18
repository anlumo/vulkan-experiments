use lazy_static::lazy_static;
use ash::{
    vk,
    extensions::{
        khr::{
            Swapchain,
            Surface,
        }
    },
    version::{
        InstanceV1_0,
    },
};
use std::{
    collections::HashSet,
    ffi::{CStr, CString},
};

use crate::swap_chain_support::SwapChainSupportDetails;

lazy_static! {
    pub static ref DEVICE_EXTENSIONS: HashSet<CString> = vec![Swapchain::name().to_owned()].into_iter().collect();
}

#[derive(Debug)]
pub struct NonDisplayDevice();
impl std::error::Error for NonDisplayDevice {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
impl std::fmt::Display for NonDisplayDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Non-display device")
    }
}

pub fn is_device_suitable(instance: &ash::Instance, device: vk::PhysicalDevice, surface_ext: &Surface, surface: &vk::SurfaceKHR) -> Result<(u32, String, SwapChainSupportDetails), Box<dyn std::error::Error>> {
    let properties = unsafe { instance.get_physical_device_properties(device) };
    let ext_props = unsafe { instance.enumerate_device_extension_properties(device) }?;
    let extensions = ext_props.into_iter().map(|props| unsafe { CStr::from_ptr(props.extension_name.as_ptr()) }.to_owned().clone()).collect();
    // let features = unsafe { instance.get_physical_device_features(device) };

    if DEVICE_EXTENSIONS.is_subset(&extensions) {
        let swap_chain_support_details = SwapChainSupportDetails::query(device, surface_ext, surface)?;
        if swap_chain_support_details.formats.is_empty() || swap_chain_support_details.present_modes.is_empty() {
            Err(Box::new(NonDisplayDevice()))
        } else {
            Ok((properties.limits.max_image_dimension2_d + match properties.device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => 1000,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 500,
                vk::PhysicalDeviceType::VIRTUAL_GPU => 250,
                _ => 0,
            }, unsafe { CStr::from_ptr(&properties.device_name as *const i8) }.to_string_lossy().to_string().clone(), swap_chain_support_details))
        }
    } else {
        Err(Box::new(NonDisplayDevice()))
    }
}
