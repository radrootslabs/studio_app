import Foundation
import RadRootsAppleSecurity

private let defaultServicePrefix = "org.radroots.app.apple-security"

private enum RadRootsAppleFFIStatus: Int32 {
    case success = 0
    case notFound = 1
    case invalidInput = 2
    case error = 3
}

@_cdecl("radroots_studio_apple_secret_store_put")
public func radroots_studio_apple_secret_store_put(
    _ servicePrefix: UnsafePointer<CChar>?,
    _ namespace: UnsafePointer<CChar>?,
    _ name: UnsafePointer<CChar>?,
    _ valuePtr: UnsafePointer<UInt8>?,
    _ valueLen: Int,
    _ accessibilityRaw: Int32,
    _ deviceLocalOnlyRaw: Int32,
    _ userPresenceRequiredRaw: Int32,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    do {
        let store = try makeStore(servicePrefix: servicePrefix)
        let key = try makeKey(namespace: namespace, name: name)
        let policy = try makePolicy(
            accessibilityRaw: accessibilityRaw,
            deviceLocalOnlyRaw: deviceLocalOnlyRaw,
            userPresenceRequiredRaw: userPresenceRequiredRaw
        )
        guard let valuePtr else {
            throw RadRootsAppleSecurityError.invalidRequest("secret value pointer cannot be null")
        }
        let value = Data(bytes: valuePtr, count: valueLen)
        try store.put(value, for: key, policy: policy)
        return RadRootsAppleFFIStatus.success.rawValue
    } catch {
        setError(error, into: errorOut)
        return statusForError(error)
    }
}

@_cdecl("radroots_studio_apple_secret_store_get")
public func radroots_studio_apple_secret_store_get(
    _ servicePrefix: UnsafePointer<CChar>?,
    _ namespace: UnsafePointer<CChar>?,
    _ name: UnsafePointer<CChar>?,
    _ valueOut: UnsafeMutablePointer<UnsafeMutablePointer<UInt8>?>?,
    _ valueLenOut: UnsafeMutablePointer<Int>?,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    do {
        guard let valueOut, let valueLenOut else {
            throw RadRootsAppleSecurityError.invalidRequest("output buffers cannot be null")
        }
        let store = try makeStore(servicePrefix: servicePrefix)
        let key = try makeKey(namespace: namespace, name: name)
        guard let value = try store.get(key) else {
            valueOut.pointee = nil
            valueLenOut.pointee = 0
            return RadRootsAppleFFIStatus.notFound.rawValue
        }

        let output = UnsafeMutablePointer<UInt8>.allocate(capacity: value.count)
        value.copyBytes(to: output, count: value.count)
        valueOut.pointee = output
        valueLenOut.pointee = value.count
        return RadRootsAppleFFIStatus.success.rawValue
    } catch {
        setError(error, into: errorOut)
        return statusForError(error)
    }
}

@_cdecl("radroots_studio_apple_secret_store_contains")
public func radroots_studio_apple_secret_store_contains(
    _ servicePrefix: UnsafePointer<CChar>?,
    _ namespace: UnsafePointer<CChar>?,
    _ name: UnsafePointer<CChar>?,
    _ containsOut: UnsafeMutablePointer<Int32>?,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    do {
        guard let containsOut else {
            throw RadRootsAppleSecurityError.invalidRequest("contains output cannot be null")
        }
        let store = try makeStore(servicePrefix: servicePrefix)
        let key = try makeKey(namespace: namespace, name: name)
        containsOut.pointee = try store.contains(key) ? 1 : 0
        return RadRootsAppleFFIStatus.success.rawValue
    } catch {
        setError(error, into: errorOut)
        return statusForError(error)
    }
}

@_cdecl("radroots_studio_apple_secret_store_delete")
public func radroots_studio_apple_secret_store_delete(
    _ servicePrefix: UnsafePointer<CChar>?,
    _ namespace: UnsafePointer<CChar>?,
    _ name: UnsafePointer<CChar>?,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    do {
        let store = try makeStore(servicePrefix: servicePrefix)
        let key = try makeKey(namespace: namespace, name: name)
        try store.delete(key)
        return RadRootsAppleFFIStatus.success.rawValue
    } catch {
        setError(error, into: errorOut)
        return statusForError(error)
    }
}

@_cdecl("radroots_studio_apple_buffer_free")
public func radroots_studio_apple_buffer_free(
    _ buffer: UnsafeMutablePointer<UInt8>?,
    _ length: Int
) {
    guard let buffer else {
        return
    }
    buffer.deallocate()
    _ = length
}

@_cdecl("radroots_studio_apple_c_string_free")
public func radroots_studio_apple_c_string_free(_ string: UnsafeMutablePointer<CChar>?) {
    string?.deallocate()
}

private func makeStore(
    servicePrefix: UnsafePointer<CChar>?
) throws -> RadRootsAppleKeychainSecretStore {
    let service = servicePrefix.map(String.init(cString:)) ?? defaultServicePrefix
    guard !service.isEmpty else {
        throw RadRootsAppleSecurityError.invalidRequest("service prefix cannot be empty")
    }
    return RadRootsAppleKeychainSecretStore(servicePrefix: service)
}

private func makeKey(
    namespace: UnsafePointer<CChar>?,
    name: UnsafePointer<CChar>?
) throws -> RadRootsAppleSecretKey {
    guard let namespace, let name else {
        throw RadRootsAppleSecurityError.invalidRequest("secret namespace and name are required")
    }
    return try RadRootsAppleSecretKey(
        namespace: String(cString: namespace),
        name: String(cString: name)
    )
}

private func makePolicy(
    accessibilityRaw: Int32,
    deviceLocalOnlyRaw: Int32,
    userPresenceRequiredRaw: Int32
) throws -> RadRootsAppleSecretAccessPolicy {
    guard let accessibility = RadRootsAppleSecretAccessibility(rawValue: accessibilityRaw) else {
        throw RadRootsAppleSecurityError.invalidRequest("invalid accessibility value")
    }
    return RadRootsAppleSecretAccessPolicy(
        accessibility: accessibility,
        deviceLocalOnly: deviceLocalOnlyRaw != 0,
        userPresenceRequired: userPresenceRequiredRaw != 0
    )
}

private func setError(
    _ error: Error,
    into errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) {
    guard let errorOut else {
        return
    }
    errorOut.pointee = duplicateCString(error.localizedDescription)
}

private func statusForError(_ error: Error) -> Int32 {
    if case RadRootsAppleSecurityError.invalidRequest = error {
        return RadRootsAppleFFIStatus.invalidInput.rawValue
    }
    return RadRootsAppleFFIStatus.error.rawValue
}

private func duplicateCString(_ value: String) -> UnsafeMutablePointer<CChar>? {
    let bytes = Array(value.utf8CString)
    let pointer = UnsafeMutablePointer<CChar>.allocate(capacity: bytes.count)
    pointer.initialize(from: bytes, count: bytes.count)
    return pointer
}
