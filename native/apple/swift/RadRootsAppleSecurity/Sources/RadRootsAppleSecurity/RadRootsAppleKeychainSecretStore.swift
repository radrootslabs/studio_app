import Foundation
import Security

public final class RadRootsAppleKeychainSecretStore: @unchecked Sendable {
    public let servicePrefix: String

    public init(servicePrefix: String = "org.radroots.app.apple-security") {
        self.servicePrefix = servicePrefix
    }

    public func put(
        _ value: Data,
        for key: RadRootsAppleSecretKey,
        policy: RadRootsAppleSecretAccessPolicy = .secureLocalSecret
    ) throws {
        try delete(key)

        let query = try writeQuery(for: key, value: value, policy: policy)
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw Self.mapSecurityStatus(status, defaultMessage: "keychain write failed")
        }
    }

    public func get(_ key: RadRootsAppleSecretKey) throws -> Data? {
        var query = baseQuery(for: key)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne

        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecItemNotFound {
            return nil
        }
        guard status == errSecSuccess else {
            throw Self.mapSecurityStatus(status, defaultMessage: "keychain read failed")
        }
        guard let data = result as? Data else {
            throw RadRootsAppleSecurityError.permanentFailure(
                "keychain read returned an invalid value type"
            )
        }
        return data
    }

    public func contains(_ key: RadRootsAppleSecretKey) throws -> Bool {
        try get(key) != nil
    }

    public func delete(_ key: RadRootsAppleSecretKey) throws {
        let status = SecItemDelete(baseQuery(for: key) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw Self.mapSecurityStatus(status, defaultMessage: "keychain delete failed")
        }
    }

    public func deleteNamespace(_ namespace: String) throws {
        guard !namespace.isEmpty else {
            throw RadRootsAppleSecurityError.invalidRequest("secret namespace cannot be empty")
        }
        let status = SecItemDelete(namespaceQuery(namespace) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw Self.mapSecurityStatus(status, defaultMessage: "keychain namespace delete failed")
        }
    }

    func baseQuery(for key: RadRootsAppleSecretKey) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: key.serviceName(servicePrefix: servicePrefix),
            kSecAttrAccount as String: key.name
        ]
    }

    func namespaceQuery(_ namespace: String) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "\(servicePrefix).\(namespace)"
        ]
    }

    func writeQuery(
        for key: RadRootsAppleSecretKey,
        value: Data,
        policy: RadRootsAppleSecretAccessPolicy
    ) throws -> [String: Any] {
        var query = baseQuery(for: key)
        query[kSecValueData as String] = value
        query[kSecAttrAccessible as String] = accessibilityConstant(for: policy)
        return query
    }

    func accessibilityConstant(for policy: RadRootsAppleSecretAccessPolicy) -> CFString {
        switch (policy.accessibility, policy.deviceLocalOnly) {
        case (.whenUnlocked, true):
            return kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        case (.whenUnlocked, false):
            return kSecAttrAccessibleWhenUnlocked
        case (.afterFirstUnlock, true):
            return kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        case (.afterFirstUnlock, false):
            return kSecAttrAccessibleAfterFirstUnlock
        }
    }

    static func mapSecurityStatus(
        _ status: OSStatus,
        defaultMessage: String
    ) -> RadRootsAppleSecurityError {
        switch status {
        case errSecAuthFailed:
            return .permissionDenied(defaultMessage)
        case errSecInteractionNotAllowed:
            return .transientFailure(defaultMessage)
        case errSecUserCanceled:
            return .userCancelled(defaultMessage)
        case errSecNotAvailable:
            return .unavailable(defaultMessage)
        default:
            return .keychainStatus(status, defaultMessage)
        }
    }
}
