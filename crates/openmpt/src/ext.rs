//! Safe rust bindings for libopenmpt extension API.
//!
//! This module provides safe wrappers around the libopenmpt extension functionality,
//! allowing for advanced features like interactive playback control and pattern visualization.

use openmpt_sys;
use std::os::raw::c_char;
use std::ptr;

use crate::module::ctls::Ctl;
use crate::module::{Logger, Module};

/// Opaque struct representing an extended module with additional functionality
pub struct ModuleExt {
    inner: *mut openmpt_sys::openmpt_module_ext,
}

impl Drop for ModuleExt {
    fn drop(&mut self) {
        unsafe {
            openmpt_sys::openmpt_module_ext_destroy(self.inner);
        }
    }
}

impl ModuleExt {
    /// Creates a new extended module from memory data
    #[allow(clippy::result_unit_err)]
    pub fn from_memory(buffer: &[u8], logger: Logger, _init_ctls: &[Ctl]) -> Result<Self, ()> {
        let module_ext_ptr = unsafe {
            openmpt_sys::openmpt_module_ext_create_from_memory(
                buffer.as_ptr() as *const _,
                buffer.len(),
                logger.log_func(),
                ptr::null_mut(),
                None, // errfunc
                ptr::null_mut(),
                ptr::null_mut(), // error
                ptr::null_mut(), // error_message
                ptr::null(),     // ctls - we'll set them manually
            )
        };

        if module_ext_ptr.is_null() {
            return Err(());
        }

        let module_ext = ModuleExt {
            inner: module_ext_ptr,
        };

        // Set each init ctl by hand - this would require a more complex implementation
        // For now, we'll skip this step

        Ok(module_ext)
    }

    /// Gets the underlying module handle
    pub fn get_module(&self) -> Module {
        let module_ptr = unsafe { openmpt_sys::openmpt_module_ext_get_module(self.inner) };

        // This creates a Module from a raw pointer without transferring ownership
        Module::from_raw_pointer(module_ptr, false)
    }

    /// Retrieves the pattern visualization interface
    pub fn get_pattern_vis_interface(&self) -> Option<PatternVisInterface<'_>> {
        let interface_id = b"pattern_vis\0";
        let mut interface_struct =
            unsafe { std::mem::zeroed::<openmpt_sys::openmpt_module_ext_interface_pattern_vis>() };

        let result = unsafe {
            openmpt_sys::openmpt_module_ext_get_interface(
                self.inner,
                interface_id.as_ptr() as *const c_char,
                &mut interface_struct as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<openmpt_sys::openmpt_module_ext_interface_pattern_vis>(),
            )
        };

        if result == 1 {
            Some(PatternVisInterface {
                inner: interface_struct,
                _module_ext: self,
            })
        } else {
            None
        }
    }

    /// Retrieves the interactive interface
    pub fn get_interactive_interface(&self) -> Option<InteractiveInterface<'_>> {
        let interface_id = b"interactive\0";
        let mut interface_struct =
            unsafe { std::mem::zeroed::<openmpt_sys::openmpt_module_ext_interface_interactive>() };

        let result = unsafe {
            openmpt_sys::openmpt_module_ext_get_interface(
                self.inner,
                interface_id.as_ptr() as *const c_char,
                &mut interface_struct as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<openmpt_sys::openmpt_module_ext_interface_interactive>(),
            )
        };

        if result == 1 {
            Some(InteractiveInterface {
                inner: interface_struct,
                _module_ext: self,
            })
        } else {
            None
        }
    }

    /// Retrieves the interactive2 interface
    pub fn get_interactive2_interface(&self) -> Option<Interactive2Interface<'_>> {
        let interface_id = b"interactive2\0";
        let mut interface_struct =
            unsafe { std::mem::zeroed::<openmpt_sys::openmpt_module_ext_interface_interactive2>() };

        let result = unsafe {
            openmpt_sys::openmpt_module_ext_get_interface(
                self.inner,
                interface_id.as_ptr() as *const c_char,
                &mut interface_struct as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<openmpt_sys::openmpt_module_ext_interface_interactive2>(),
            )
        };

        if result == 1 {
            Some(Interactive2Interface {
                inner: interface_struct,
                _module_ext: self,
            })
        } else {
            None
        }
    }

    /// Retrieves the interactive3 interface
    pub fn get_interactive3_interface(&self) -> Option<Interactive3Interface<'_>> {
        let interface_id = b"interactive3\0";
        let mut interface_struct =
            unsafe { std::mem::zeroed::<openmpt_sys::openmpt_module_ext_interface_interactive3>() };

        let result = unsafe {
            openmpt_sys::openmpt_module_ext_get_interface(
                self.inner,
                interface_id.as_ptr() as *const c_char,
                &mut interface_struct as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<openmpt_sys::openmpt_module_ext_interface_interactive3>(),
            )
        };

        if result == 1 {
            Some(Interactive3Interface {
                inner: interface_struct,
                _module_ext: self,
            })
        } else {
            None
        }
    }

    /// Render audio data as interleaved stereo.
    ///
    /// ### Parameters
    /// * `sample_rate` : Sample rate to render output. Should be in [8000,192000], but this is not enforced.
    /// * `interleaved_stereo` : Pointer to a buffer for the interleaved stereo output (order : L,R) that will receive an amount of audio frames equal to its capacity divided by the number of channels.
    ///
    /// ### Returns
    /// The number of frames actually rendered (up to half of the buffer's capacity), or 0 if the end of song has been reached.
    pub fn read_interleaved_stereo(
        &self,
        sample_rate: i32,
        interleaved_stereo: &mut [i16],
    ) -> usize {
        let count = interleaved_stereo.len() >> 1; // Buffer needs to be of at least size count*2

        let raw_module = unsafe { openmpt_sys::openmpt_module_ext_get_module(self.inner) };

        unsafe {
            openmpt_sys::openmpt_module_read_interleaved_stereo(
                raw_module,
                sample_rate,
                count,
                interleaved_stereo.as_mut_ptr(),
            )
        }
    }

    /// Get current song position in seconds.
    ///
    /// ### Returns
    /// Current song position in seconds.
    pub fn get_position_seconds(&self) -> f64 {
        let raw_module = unsafe { openmpt_sys::openmpt_module_ext_get_module(self.inner) };

        unsafe { openmpt_sys::openmpt_module_get_position_seconds(raw_module) }
    }

    /// Get the approximate song duration in seconds.
    ///
    /// ### Returns
    /// Approximate duration of current sub-song in seconds.
    pub fn get_duration_seconds(&self) -> f64 {
        let raw_module = unsafe { openmpt_sys::openmpt_module_ext_get_module(self.inner) };

        unsafe { openmpt_sys::openmpt_module_get_duration_seconds(raw_module) }
    }
}

/// Pattern visualization interface wrapper
pub struct PatternVisInterface<'a> {
    inner: openmpt_sys::openmpt_module_ext_interface_pattern_vis,
    _module_ext: &'a ModuleExt,
}

impl<'a> PatternVisInterface<'a> {
    /// Get pattern command type for pattern highlighting (volume column)
    pub fn get_pattern_row_channel_volume_effect_type(
        &self,
        module_ext: &ModuleExt,
        pattern: i32,
        row: i32,
        channel: i32,
    ) -> i32 {
        if let Some(func) = self.inner.get_pattern_row_channel_volume_effect_type {
            unsafe { func(module_ext.inner, pattern, row, channel) }
        } else {
            0 // Default to unknown
        }
    }

    /// Get pattern command type for pattern highlighting (effect column)
    pub fn get_pattern_row_channel_effect_type(
        &self,
        module_ext: &ModuleExt,
        pattern: i32,
        row: i32,
        channel: i32,
    ) -> i32 {
        if let Some(func) = self.inner.get_pattern_row_channel_effect_type {
            unsafe { func(module_ext.inner, pattern, row, channel) }
        } else {
            0 // Default to unknown
        }
    }
}

/// Interactive interface wrapper
pub struct InteractiveInterface<'a> {
    inner: openmpt_sys::openmpt_module_ext_interface_interactive,
    _module_ext: &'a ModuleExt,
}

impl<'a> InteractiveInterface<'a> {
    /// Set the current ticks per row (speed)
    pub fn set_current_speed(&self, module_ext: &ModuleExt, speed: i32) -> bool {
        if let Some(func) = self.inner.set_current_speed {
            unsafe { func(module_ext.inner, speed) == 1 }
        } else {
            false
        }
    }

    /// Set the current module tempo
    pub fn set_current_tempo(&self, module_ext: &ModuleExt, tempo: i32) -> bool {
        if let Some(func) = self.inner.set_current_tempo {
            unsafe { func(module_ext.inner, tempo) == 1 }
        } else {
            false
        }
    }

    /// Set the current module tempo factor without affecting playback pitch
    pub fn set_tempo_factor(&self, module_ext: &ModuleExt, factor: f64) -> bool {
        if let Some(func) = self.inner.set_tempo_factor {
            unsafe { func(module_ext.inner, factor) == 1 }
        } else {
            false
        }
    }

    /// Get the current module tempo factor
    pub fn get_tempo_factor(&self, module_ext: &ModuleExt) -> f64 {
        if let Some(func) = self.inner.get_tempo_factor {
            unsafe { func(module_ext.inner) }
        } else {
            1.0 // Default to no change
        }
    }

    /// Set the current module pitch factor without affecting playback speed
    pub fn set_pitch_factor(&self, module_ext: &ModuleExt, factor: f64) -> bool {
        if let Some(func) = self.inner.set_pitch_factor {
            unsafe { func(module_ext.inner, factor) == 1 }
        } else {
            false
        }
    }

    /// Get the current module pitch factor
    pub fn get_pitch_factor(&self, module_ext: &ModuleExt) -> f64 {
        if let Some(func) = self.inner.get_pitch_factor {
            unsafe { func(module_ext.inner) }
        } else {
            1.0 // Default to no change
        }
    }

    /// Set the current global volume
    pub fn set_global_volume(&self, module_ext: &ModuleExt, volume: f64) -> bool {
        if let Some(func) = self.inner.set_global_volume {
            unsafe { func(module_ext.inner, volume) == 1 }
        } else {
            false
        }
    }

    /// Get the current global volume
    pub fn get_global_volume(&self, module_ext: &ModuleExt) -> f64 {
        if let Some(func) = self.inner.get_global_volume {
            unsafe { func(module_ext.inner) }
        } else {
            0.0 // Default
        }
    }

    /// Set the current channel volume for a channel
    pub fn set_channel_volume(&self, module_ext: &ModuleExt, channel: i32, volume: f64) -> bool {
        if let Some(func) = self.inner.set_channel_volume {
            unsafe { func(module_ext.inner, channel, volume) == 1 }
        } else {
            false
        }
    }

    /// Get the current channel volume for a channel
    pub fn get_channel_volume(&self, module_ext: &ModuleExt, channel: i32) -> f64 {
        if let Some(func) = self.inner.get_channel_volume {
            unsafe { func(module_ext.inner, channel) }
        } else {
            0.0 // Default
        }
    }

    /// Set the current mute status for a channel
    pub fn set_channel_mute_status(
        &self,
        module_ext: &ModuleExt,
        channel: i32,
        mute: bool,
    ) -> bool {
        if let Some(func) = self.inner.set_channel_mute_status {
            unsafe { func(module_ext.inner, channel, if mute { 1 } else { 0 }) == 1 }
        } else {
            false
        }
    }

    /// Get the current mute status for a channel
    pub fn get_channel_mute_status(&self, module_ext: &ModuleExt, channel: i32) -> Option<bool> {
        if let Some(func) = self.inner.get_channel_mute_status {
            let result = unsafe { func(module_ext.inner, channel) };

            match result {
                1 => Some(true),
                0 => Some(false),
                _ => None, // Invalid channel
            }
        } else {
            None
        }
    }

    /// Set the current mute status for an instrument
    pub fn set_instrument_mute_status(
        &self,
        module_ext: &ModuleExt,
        instrument: i32,
        mute: bool,
    ) -> bool {
        if let Some(func) = self.inner.set_instrument_mute_status {
            unsafe { func(module_ext.inner, instrument, if mute { 1 } else { 0 }) == 1 }
        } else {
            false
        }
    }

    /// Get the current mute status for an instrument
    pub fn get_instrument_mute_status(
        &self,
        module_ext: &ModuleExt,
        instrument: i32,
    ) -> Option<bool> {
        if let Some(func) = self.inner.get_instrument_mute_status {
            let result = unsafe { func(module_ext.inner, instrument) };

            match result {
                1 => Some(true),
                0 => Some(false),
                _ => None, // Invalid instrument
            }
        } else {
            None
        }
    }

    /// Play a note using the specified instrument
    pub fn play_note(
        &self,
        module_ext: &ModuleExt,
        instrument: i32,
        note: i32,
        volume: f64,
        panning: f64,
    ) -> Option<i32> {
        if let Some(func) = self.inner.play_note {
            let channel = unsafe { func(module_ext.inner, instrument, note, volume, panning) };

            if channel >= 0 {
                Some(channel)
            } else {
                None // Failed to allocate channel
            }
        } else {
            None
        }
    }

    /// Stop the note playing on the specified channel
    pub fn stop_note(&self, module_ext: &ModuleExt, channel: i32) -> bool {
        if let Some(func) = self.inner.stop_note {
            unsafe { func(module_ext.inner, channel) == 1 }
        } else {
            false
        }
    }
}

/// Interactive2 interface wrapper
pub struct Interactive2Interface<'a> {
    inner: openmpt_sys::openmpt_module_ext_interface_interactive2,
    _module_ext: &'a ModuleExt,
}

impl<'a> Interactive2Interface<'a> {
    /// Sends a key-off command for the note playing on the specified channel
    pub fn note_off(&self, module_ext: &ModuleExt, channel: i32) -> bool {
        if let Some(func) = self.inner.note_off {
            unsafe { func(module_ext.inner, channel) == 1 }
        } else {
            false
        }
    }

    /// Sends a note fade command for the note playing on the specified channel
    pub fn note_fade(&self, module_ext: &ModuleExt, channel: i32) -> bool {
        if let Some(func) = self.inner.note_fade {
            unsafe { func(module_ext.inner, channel) == 1 }
        } else {
            false
        }
    }

    /// Set the current panning for a channel
    pub fn set_channel_panning(&self, module_ext: &ModuleExt, channel: i32, panning: f64) -> bool {
        if let Some(func) = self.inner.set_channel_panning {
            unsafe { func(module_ext.inner, channel, panning) == 1 }
        } else {
            false
        }
    }

    /// Get the current panning position for a channel
    pub fn get_channel_panning(&self, module_ext: &ModuleExt, channel: i32) -> f64 {
        if let Some(func) = self.inner.get_channel_panning {
            unsafe { func(module_ext.inner, channel) }
        } else {
            0.0 // Center
        }
    }

    /// Set the finetune for the currently playing note on a channel
    pub fn set_note_finetune(&self, module_ext: &ModuleExt, channel: i32, finetune: f64) -> bool {
        if let Some(func) = self.inner.set_note_finetune {
            unsafe { func(module_ext.inner, channel, finetune) == 1 }
        } else {
            false
        }
    }

    /// Get the finetune for the currently playing note on a channel
    pub fn get_note_finetune(&self, module_ext: &ModuleExt, channel: i32) -> f64 {
        if let Some(func) = self.inner.get_note_finetune {
            unsafe { func(module_ext.inner, channel) }
        } else {
            0.0 // No finetune
        }
    }
}

/// Interactive3 interface wrapper
pub struct Interactive3Interface<'a> {
    inner: openmpt_sys::openmpt_module_ext_interface_interactive3,
    _module_ext: &'a ModuleExt,
}

impl<'a> Interactive3Interface<'a> {
    /// Set the current module tempo (version 2)
    pub fn set_current_tempo2(&self, module_ext: &ModuleExt, tempo: f64) -> bool {
        if let Some(func) = self.inner.set_current_tempo2 {
            unsafe { func(module_ext.inner, tempo) == 1 }
        } else {
            false
        }
    }
}
