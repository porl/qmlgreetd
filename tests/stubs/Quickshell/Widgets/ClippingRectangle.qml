// Offscreen stand-in for Quickshell's ClippingRectangle. The real type is a
// linktarget plugin that only loads inside Quickshell, so the test tree stubs
// it; the login card's avatar needs a clipped rounded rectangle and nothing of
// the real type's rendering.
import QtQuick

Rectangle {
    clip: true
}
