use ash::{
    vk,
    version::{
        InstanceV1_0,
    },
};

#[derive(Default)]
pub struct QueueFamilyIndices {
    pub graphics: Option<u32>,
    pub compute: Option<u32>,
    pub transfer: Option<u32>,
    pub sparse_binding: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn find(instance: &ash::Instance, device: vk::PhysicalDevice) -> Self {
        let mut result = Self::default();

        for (idx, properties) in unsafe { instance.get_physical_device_queue_family_properties(device) }.into_iter().enumerate() {
            if properties.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                result.graphics = Some(idx as u32);
            } else if properties.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                result.compute = Some(idx as u32);
            } else if properties.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                result.transfer = Some(idx as u32);
            } else if properties.queue_flags.contains(vk::QueueFlags::SPARSE_BINDING) {
                result.sparse_binding = Some(idx as u32);
            }
        }
        
        result
    }

    pub fn is_device_suitable(&self) -> bool {
        self.graphics.is_some()
    }
}
