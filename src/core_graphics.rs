//! CoreGraphics FFI bindings — CGEventTap, CGEvent, CGWindowLevel.
#![macro_use]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi::c_void;
use crate::core_foundation::CFMachPortRef;

#[repr(u32)]
#[allow(dead_code)]
pub enum CGEventTapLocation {
    kCGHIDEventTap = 0,
    kCGSessionEventTap,
    kCGAnnotatedSessionEventTap,
}

#[repr(u32)]
#[allow(dead_code)]
pub enum CGEventTapPlacement {
    kCGHeadInsertEventTap = 0,
    kCGTailAppendEventTap,
}

#[repr(u32)]
#[allow(dead_code)]
pub enum CGEventTapOptions {
    kCGEventTapOptionDefault = 0x00000000,
    kCGEventTapOptionListenOnly = 0x00000001,
}

#[repr(u32)]
#[allow(dead_code)]
pub enum CGEventType {
    kCGEventNull = 0,
    kCGEventLeftMouseDown = 1,
    kCGEventLeftMouseUp = 2,
    kCGEventRightMouseDown = 3,
    kCGEventRightMouseUp = 4,
    kCGEventMouseMoved = 5,
    kCGEventLeftMouseDragged = 6,
    kCGEventRightMouseDragged = 7,
    kCGEventKeyDown = 10,
    kCGEventKeyUp = 11,
    kCGEventFlagsChanged = 12,
    kCGEventScrollWheel = 22,
    kCGEventTabletPointer = 23,
    kCGEventTabletProximity = 24,
    kCGEventOtherMouseDown = 25,
    kCGEventOtherMouseUp = 26,
    kCGEventOtherMouseDragged = 27,
    kCGEventTapDisabledByTimeout = 0xFFFFFFFE,
    kCGEventTapDisabledByUserInput = 0xFFFFFFFF,
}

#[repr(i32)]
pub enum CGWindowLevelKey {
    kCGBaseWindowLevelKey = 0,
    kCGMinimumWindowLevelKey,
    kCGDesktopWindowLevelKey,
    kCGBackstopMenuLevelKey,
    kCGNormalWindowLevelKey,
    kCGFloatingWindowLevelKey,
    kCGTornOffMenuWindowLevelKey,
    kCGDockWindowLevelKey,
    kCGMainMenuWindowLevelKey,
    kCGStatusWindowLevelKey,
    kCGModalPanelWindowLevelKey,
    kCGPopUpMenuWindowLevelKey,
    kCGDraggingWindowLevelKey,
    kCGScreenSaverWindowLevelKey,
    kCGMaximumWindowLevelKey,
    kCGOverlayWindowLevelKey,
    kCGHelpWindowLevelKey,
    kCGUtilityWindowLevelKey,
    kCGDesktopIconWindowLevelKey,
    kCGCursorWindowLevelKey,
    kCGAssistiveTechHighWindowLevelKey,
    kCGNumberOfWindowLevelKeys,
}

#[repr(u32)]
#[allow(dead_code)]
pub enum CGEventField {
    kCGMouseEventNumber = 0,
    kCGMouseEventClickState = 1,
    kCGMouseEventPressure = 2,
    kCGMouseEventButtonNumber = 3,
    kCGMouseEventDeltaX = 4,
    kCGMouseEventDeltaY = 5,
    kCGMouseEventInstantMouser = 6,
    kCGMouseEventSubtype = 7,
    kCGKeyboardEventAutorepeat = 8,
    kCGKeyboardEventKeycode = 9,
    kCGKeyboardEventKeyboardType = 10,
    kCGScrollWheelEventDeltaAxis1 = 11,
    kCGScrollWheelEventDeltaAxis2 = 12,
    kCGScrollWheelEventDeltaAxis3 = 13,
    kCGScrollWheelEventFixedPtDeltaAxis1 = 93,
    kCGScrollWheelEventFixedPtDeltaAxis2 = 94,
    kCGScrollWheelEventFixedPtDeltaAxis3 = 95,
    kCGScrollWheelEventPointDeltaAxis1 = 96,
    kCGScrollWheelEventPointDeltaAxis2 = 97,
    kCGScrollWheelEventPointDeltaAxis3 = 98,
    kCGScrollWheelEventScrollPhase = 99,
    kCGScrollWheelEventScrollCount = 100,
    kCGScrollWheelEventMomentumPhase = 123,
    kCGScrollWheelEventInstantMouser = 14,
    kCGTabletEventPointX = 15,
    kCGTabletEventPointY = 16,
    kCGTabletEventPointZ = 17,
    kCGTabletEventPointButtons = 18,
    kCGTabletEventPointPressure = 19,
    kCGTabletEventTiltX = 20,
    kCGTabletEventTiltY = 21,
    kCGTabletEventRotation = 22,
    kCGTabletEventTangentialPressure = 23,
    kCGTabletEventDeviceID = 24,
    kCGTabletEventVendor1 = 25,
    kCGTabletEventVendor2 = 26,
    kCGTabletEventVendor3 = 27,
    kCGTabletProximityEventVendorID = 28,
    kCGTabletProximityEventTabletID = 29,
    kCGTabletProximityEventPointerID = 30,
    kCGTabletProximityEventDeviceID = 31,
    kCGTabletProximityEventSystemTabletID = 32,
    kCGTabletProximityEventVendorPointerType = 33,
    kCGTabletProximityEventVendorPointerSerialNumber = 34,
    kCGTabletProximityEventVendorUniqueID = 35,
    kCGTabletProximityEventCapabilityMask = 36,
    kCGTabletProximityEventPointerType = 37,
    kCGTabletProximityEventEnterProximity = 38,
    kCGEventTargetProcessSerialNumber = 39,
    kCGEventTargetUnixProcessID = 40,
    kCGEventSourceUnixProcessID = 41,
    kCGEventSourceUserData = 42,
    kCGEventSourceUserID = 43,
    kCGEventSourceGroupID = 44,
    kCGEventSourceStateID = 45,
    kCGScrollWheelEventIsContinuous = 88,
    kCGMouseEventWindowUnderMousePointer = 91,
    kCGMouseEventWindowUnderMousePointerThatCanHandleThisEvent = 92,
}

pub type CGEventMask = u64;
pub type CGEventTapProxy = *mut c_void;
pub type CGEventRef = *mut c_void;
pub type CGEventTapCallBack = extern "C" fn(
    proxy: CGEventTapProxy,
    etype: CGEventType,
    event: CGEventRef,
    refcon: *mut c_void,
) -> CGEventRef;

#[macro_export]
macro_rules! CGEventMaskBit {
    ($eventType:expr) => {
        1 << ($eventType as CGEventMask)
    };
}

pub const kCGEventMaskForAllEvents: u64 = !0;

#[link(name = "CoreGraphics", kind = "framework")]
extern "system" {
    pub fn CGEventTapCreate(
        tap: CGEventTapLocation,
        place: CGEventTapPlacement,
        options: CGEventTapOptions,
        eventsOfInterest: CGEventMask,
        callback: CGEventTapCallBack,
        userInfo: *mut c_void,
    ) -> CFMachPortRef;

    pub fn CGEventGetIntegerValueField(event: CGEventRef, field: CGEventField) -> i64;
    pub fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    pub fn CGEventTapIsEnabled(tap: CFMachPortRef) -> bool;
    pub fn CGWindowLevelForKey(key: CGWindowLevelKey) -> i32;
}
