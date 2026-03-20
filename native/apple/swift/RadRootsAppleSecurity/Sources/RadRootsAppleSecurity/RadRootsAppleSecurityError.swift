import Foundation
import Security

public enum RadRootsAppleSecurityError: Error, Sendable {
    case invalidRequest(String)
    case permissionDenied(String)
    case userCancelled(String)
    case unavailable(String)
    case transientFailure(String)
    case permanentFailure(String)
    case keychainStatus(OSStatus, String)
}

extension RadRootsAppleSecurityError: LocalizedError {
    public var errorDescription: String? {
        switch self {
        case let .invalidRequest(message),
             let .permissionDenied(message),
             let .userCancelled(message),
             let .unavailable(message),
             let .transientFailure(message),
             let .permanentFailure(message):
            return message
        case let .keychainStatus(status, message):
            return "\(message) (status \(status))"
        }
    }
}
