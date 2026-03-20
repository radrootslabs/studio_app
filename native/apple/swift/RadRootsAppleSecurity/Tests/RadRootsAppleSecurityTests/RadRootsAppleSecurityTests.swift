import Foundation
import Security
@testable import RadRootsAppleSecurity
@testable import RadRootsAppleSecurityFFI
import Testing

struct RadRootsAppleSecurityTests {
    @Test
    func secretKeyRejectsEmptyNamespace() throws {
        #expect(throws: RadRootsAppleSecurityError.self) {
            _ = try RadRootsAppleSecretKey(namespace: "", name: "secret")
        }
    }

    @Test
    func secretKeyRejectsEmptyName() throws {
        #expect(throws: RadRootsAppleSecurityError.self) {
            _ = try RadRootsAppleSecretKey(namespace: "nostr", name: "")
        }
    }

    @Test
    func baseQueryUsesStableServicePrefixAndAccountName() throws {
        let store = RadRootsAppleKeychainSecretStore(servicePrefix: "org.radroots.app.nostr")
        let key = try RadRootsAppleSecretKey(namespace: "accounts", name: "account-1")

        let query = store.baseQuery(for: key)

        #expect(query[kSecAttrService as String] as? String == "org.radroots.app.nostr.accounts")
        #expect(query[kSecAttrAccount as String] as? String == "account-1")
        #expect(query[kSecClass as String] != nil)
    }

    @Test
    func secureLocalSecretDefaultsToDeviceLocalWhenUnlocked() {
        let policy = RadRootsAppleSecretAccessPolicy.secureLocalSecret

        #expect(policy.accessibility == .whenUnlocked)
        #expect(policy.deviceLocalOnly)
        #expect(!policy.userPresenceRequired)
    }

    @Test
    func accessibilityConstantMatchesPolicy() {
        let store = RadRootsAppleKeychainSecretStore()
        let localPolicy = RadRootsAppleSecretAccessPolicy(
            accessibility: .whenUnlocked,
            deviceLocalOnly: true,
            userPresenceRequired: false
        )
        let syncedPolicy = RadRootsAppleSecretAccessPolicy(
            accessibility: .afterFirstUnlock,
            deviceLocalOnly: false,
            userPresenceRequired: false
        )

        #expect(store.accessibilityConstant(for: localPolicy) == kSecAttrAccessibleWhenUnlockedThisDeviceOnly)
        #expect(store.accessibilityConstant(for: syncedPolicy) == kSecAttrAccessibleAfterFirstUnlock)
    }
}
