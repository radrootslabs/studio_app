import Foundation

public enum RadRootsAppleSecretAccessibility: Int32, Sendable {
    case whenUnlocked = 0
    case afterFirstUnlock = 1
}

public struct RadRootsAppleSecretAccessPolicy: Sendable, Equatable {
    public let accessibility: RadRootsAppleSecretAccessibility
    public let deviceLocalOnly: Bool
    public let userPresenceRequired: Bool

    public init(
        accessibility: RadRootsAppleSecretAccessibility,
        deviceLocalOnly: Bool,
        userPresenceRequired: Bool
    ) {
        self.accessibility = accessibility
        self.deviceLocalOnly = deviceLocalOnly
        self.userPresenceRequired = userPresenceRequired
    }

    public static let secureLocalSecret = Self(
        accessibility: .whenUnlocked,
        deviceLocalOnly: true,
        userPresenceRequired: false
    )
}
