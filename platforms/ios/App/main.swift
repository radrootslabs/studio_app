import Foundation
import UIKit

@_cdecl("radroots_ios_clipboard_text_copy")
func radroots_ios_clipboard_text_copy() -> UnsafeMutablePointer<CChar>? {
    guard let clipboardText = UIPasteboard.general.string?
        .trimmingCharacters(in: .whitespacesAndNewlines),
        !clipboardText.isEmpty
    else {
        return nil
    }

    return clipboardText.withCString { value in
        strdup(value)
    }
}

@_cdecl("radroots_ios_string_free")
func radroots_ios_string_free(_ value: UnsafeMutablePointer<CChar>?) {
    free(value)
}

_ = radroots_ios_run()
