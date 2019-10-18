use ash::{
    vk,
    extensions::{
        khr::Surface,
    },
};
use std::cmp::{min, max};

#[derive(Default)]
pub struct SwapChainSupportDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapChainSupportDetails {
    pub fn query(device: vk::PhysicalDevice, surface_ext: &Surface, surface: &vk::SurfaceKHR) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            capabilities: unsafe { surface_ext.get_physical_device_surface_capabilities(device, *surface) }?,
            formats: unsafe { surface_ext.get_physical_device_surface_formats(device, *surface) }?,
            present_modes: unsafe { surface_ext.get_physical_device_surface_present_modes(device, *surface) }?,
        })
    }

    pub fn choose_format(&self) -> &vk::SurfaceFormatKHR {
        self.formats.iter().find(|format| format.format == vk::Format::B8G8R8A8_UNORM && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR).unwrap_or(&self.formats[0])
    }
    pub fn choose_present_mode(&self) -> vk::PresentModeKHR {
        self.present_modes.iter().find(|present_mode| **present_mode == vk::PresentModeKHR::MAILBOX).cloned()
            .unwrap_or(vk::PresentModeKHR::FIFO)
    }
    pub fn choose_swap_extent(&self, width: u32, height: u32) -> vk::Extent2D {
        if self.capabilities.current_extent.width != std::u32::MAX {
            self.capabilities.current_extent
        } else {
            let actual_extent = vk::Extent2D::builder()
                .width(max(self.capabilities.min_image_extent.width, min(self.capabilities.max_image_extent.width, width)))
                .height(max(self.capabilities.min_image_extent.height, min(self.capabilities.max_image_extent.height, height)));

            *actual_extent
        }
    }
}
