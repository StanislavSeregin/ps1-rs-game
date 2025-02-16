#[derive(Clone, PartialEq, Debug)]
pub enum TransferDirection {
    DeviceToRam = 0,
    RamToDevice = 1,
}

#[derive(Clone, PartialEq, Debug)]
pub enum MardIncrementPerStep {
    Forward4 = 0,
    Backward4 = 1,
}

#[derive(Clone, PartialEq, Debug)]
pub enum TransferMode {
    /// transfer data all at once after DREQ is first asserted
    Burst = 0,
    /// split data into blocks, transfer next block whenever DREQ is asserted
    Slice = 1,
    LinkedList = 2,
    Reserved = 3
}

#[derive(Clone, PartialEq, Debug)]
pub enum StartTransfer {
    StoppedOrCompleted = 0,
    StartOrBusy = 1
}

pub struct DmaChannelControl {
    /// Transfer direction
    pub transfer_direction: TransferDirection,
    /// MADR increment per step
    pub mard_increment_per_step: MardIncrementPerStep,
    /// When true:
    /// -Burst mode: enable "chopping" (cycle stealing by CPU)
    /// -Slice mode: Causes DMA to hang
    /// -Linked-list mode: Transfer header before data?
    pub extended_mode_1: bool,
    /// Transfer mode (SyncMode)
    pub transfer_mode: TransferMode,
    /// Chopping DMA window size (1 << N words)
    pub chopping_dma_window_size: u8,
    /// Chopping CPU window size (1 << N cycles)
    pub chopping_cpu_window_size: u8,
    /// Start transfer
    pub start_transfer: StartTransfer,
    /// Force transfer start without waiting for DREQ
    pub force_transfer_start_without_waiting_for_dreq: bool,
    /// In forced-burst mode, pauses transfer while set.
    /// In other modes, stops bit 28 from being cleared after a slice is transferred.
    /// No effect when transfer was caused by a DREQ.
    pub extended_mode_2: bool,
    /// Perform bus snooping (allows DMA to read from -nonexistent- cache?)
    pub perform_bus_snooping: bool
}

impl DmaChannelControl {
    pub fn new() -> Self {
        Self {
            transfer_direction: TransferDirection::RamToDevice,
            mard_increment_per_step: MardIncrementPerStep::Forward4,
            extended_mode_1: false,
            transfer_mode: TransferMode::Slice,
            chopping_dma_window_size: 0,
            chopping_cpu_window_size: 0,
            start_transfer: StartTransfer::StartOrBusy,
            force_transfer_start_without_waiting_for_dreq: false, 
            extended_mode_2: false,
            perform_bus_snooping: false
        }
    }

    pub fn to_u32(&self) -> u32 {
        let mut value = 0;
        value |= (self.transfer_direction.clone() as u32) << 0;
        value |= (self.mard_increment_per_step.clone() as u32) << 1;
        value |= (self.extended_mode_1.clone() as u32) << 8;
        value |= (self.transfer_mode.clone() as u32) << 9;
        value |= (self.chopping_dma_window_size as u32) << 16;
        value |= (self.chopping_cpu_window_size as u32) << 20;
        value |= (self.start_transfer.clone() as u32) << 24;
        value |= (self.force_transfer_start_without_waiting_for_dreq as u32) << 28;
        value |= (self.extended_mode_2 as u32) << 29;
        value |= (self.perform_bus_snooping as u32) << 30;
        value
    }
}