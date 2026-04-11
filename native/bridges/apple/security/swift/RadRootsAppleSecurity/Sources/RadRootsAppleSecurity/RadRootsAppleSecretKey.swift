import Foundation

public struct RadRootsAppleSecretKey: Hashable, Sendable {
    public let namespace: String
    public let name: String

    public init(namespace: String, name: String) throws {
        guard !namespace.isEmpty else {
            throw RadRootsAppleSecurityError.invalidRequest("secret namespace cannot be empty")
        }
        guard !name.isEmpty else {
            throw RadRootsAppleSecurityError.invalidRequest("secret name cannot be empty")
        }
        self.namespace = namespace
        self.name = name
    }

    func serviceName(servicePrefix: String) -> String {
        "\(servicePrefix).\(namespace)"
    }
}
