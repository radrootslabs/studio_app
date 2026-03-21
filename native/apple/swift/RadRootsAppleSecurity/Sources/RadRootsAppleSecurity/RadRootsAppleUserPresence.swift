import Foundation

#if canImport(LocalAuthentication)
import LocalAuthentication
#endif

public enum RadRootsAppleUserPresencePolicy: Sendable {
    case deviceOwnerAuthentication
    case deviceOwnerAuthenticationWithBiometrics
}

public enum RadRootsAppleUserPresenceSupport: Sendable {
    case none
    case deviceCredential
    case biometricsOrDeviceCredential
}

public enum RadRootsAppleBiometryKind: Sendable {
    case none
    case touchID
    case faceID
    case opticID
    case unknown
}

public struct RadRootsAppleUserPresenceStatus: Sendable {
    public let support: RadRootsAppleUserPresenceSupport
    public let biometryKind: RadRootsAppleBiometryKind
    public let canEvaluateDeviceCredential: Bool
    public let canEvaluateBiometrics: Bool

    public init(
        support: RadRootsAppleUserPresenceSupport,
        biometryKind: RadRootsAppleBiometryKind,
        canEvaluateDeviceCredential: Bool,
        canEvaluateBiometrics: Bool
    ) {
        self.support = support
        self.biometryKind = biometryKind
        self.canEvaluateDeviceCredential = canEvaluateDeviceCredential
        self.canEvaluateBiometrics = canEvaluateBiometrics
    }
}

public actor RadRootsAppleUserPresence {
    public init() {}

    public static func verifySync(
        reason: String,
        policy: RadRootsAppleUserPresencePolicy = .deviceOwnerAuthentication
    ) throws -> Bool {
        #if canImport(LocalAuthentication)
        let context = LAContext()
        let lock = NSLock()
        let semaphore = DispatchSemaphore(value: 0)
        var result: Result<Bool, Error>?

        context.evaluatePolicy(
            Self.makePolicy(policy),
            localizedReason: reason
        ) { success, error in
            lock.lock()
            if let error {
                result = .failure(Self.adapt(error: error))
            } else {
                result = .success(success)
            }
            lock.unlock()
            semaphore.signal()
        }

        semaphore.wait()

        lock.lock()
        defer { lock.unlock() }
        return try result?.get() ?? {
            throw RadRootsAppleSecurityError.transientFailure(
                "local authentication did not return a result"
            )
        }()
        #else
        throw RadRootsAppleSecurityError.unavailable("local authentication is unavailable")
        #endif
    }

    public func currentStatus() -> RadRootsAppleUserPresenceStatus {
        #if canImport(LocalAuthentication)
        let context = LAContext()
        return Self.makeStatus(context: context)
        #else
        return RadRootsAppleUserPresenceStatus(
            support: .none,
            biometryKind: .none,
            canEvaluateDeviceCredential: false,
            canEvaluateBiometrics: false
        )
        #endif
    }

    public func verify(
        reason: String,
        policy: RadRootsAppleUserPresencePolicy = .deviceOwnerAuthentication
    ) async throws -> Bool {
        #if canImport(LocalAuthentication)
        let context = LAContext()
        return try await withCheckedThrowingContinuation { continuation in
            context.evaluatePolicy(
                Self.makePolicy(policy),
                localizedReason: reason
            ) { success, error in
                if let error {
                    continuation.resume(throwing: Self.adapt(error: error))
                } else {
                    continuation.resume(returning: success)
                }
            }
        }
        #else
        throw RadRootsAppleSecurityError.unavailable("local authentication is unavailable")
        #endif
    }

    #if canImport(LocalAuthentication)
    private static func makePolicy(_ policy: RadRootsAppleUserPresencePolicy) -> LAPolicy {
        switch policy {
        case .deviceOwnerAuthentication:
            return .deviceOwnerAuthentication
        case .deviceOwnerAuthenticationWithBiometrics:
            return .deviceOwnerAuthenticationWithBiometrics
        }
    }

    private static func makeStatus(context: LAContext) -> RadRootsAppleUserPresenceStatus {
        var biometricsError: NSError?
        let canEvaluateBiometrics = context.canEvaluatePolicy(
            .deviceOwnerAuthenticationWithBiometrics,
            error: &biometricsError
        )

        var deviceCredentialError: NSError?
        let canEvaluateDeviceCredential = context.canEvaluatePolicy(
            .deviceOwnerAuthentication,
            error: &deviceCredentialError
        )

        let support: RadRootsAppleUserPresenceSupport
        if canEvaluateBiometrics {
            support = .biometricsOrDeviceCredential
        } else if canEvaluateDeviceCredential {
            support = .deviceCredential
        } else {
            support = .none
        }

        return RadRootsAppleUserPresenceStatus(
            support: support,
            biometryKind: makeBiometryKind(context.biometryType),
            canEvaluateDeviceCredential: canEvaluateDeviceCredential,
            canEvaluateBiometrics: canEvaluateBiometrics
        )
    }

    private static func makeBiometryKind(_ biometryType: LABiometryType) -> RadRootsAppleBiometryKind {
        switch biometryType {
        case .none:
            return .none
        case .touchID:
            return .touchID
        case .faceID:
            return .faceID
        case .opticID:
            return .opticID
        @unknown default:
            return .unknown
        }
    }

    private static func adapt(error: Error) -> RadRootsAppleSecurityError {
        if let laError = error as? LAError {
            switch laError.code {
            case .userCancel, .userFallback:
                return .userCancelled(laError.localizedDescription)
            case .appCancel, .systemCancel, .notInteractive:
                return .transientFailure(laError.localizedDescription)
            case .biometryNotAvailable, .biometryNotEnrolled, .passcodeNotSet:
                return .unavailable(laError.localizedDescription)
            case .authenticationFailed:
                return .permissionDenied(laError.localizedDescription)
            default:
                return .permanentFailure(laError.localizedDescription)
            }
        }

        return .permanentFailure(error.localizedDescription)
    }
    #endif
}
