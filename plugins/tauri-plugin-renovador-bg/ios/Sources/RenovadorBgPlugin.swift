import BackgroundTasks
import Foundation
import Security
import SwiftRs
import Tauri
import UIKit
import WebKit

/// Identificador da tarefa; precisa constar em `BGTaskSchedulerPermittedIdentifiers` do Info.plist do app.
let renewTaskIdentifier = "br.edu.uftm.editora.renovador.refresh"

class ScheduleNextArgs: Decodable {
    let at: String
}

/// Esqueleto do Keychain: guarda a chave do banco (32 bytes) como item genérico.
enum DbKeyKeychain {
    static let service = "br.edu.uftm.editora.renovador"
    static let account = "db_key"

    static func getOrCreate() throws -> Data {
        if let existing = try load() { return existing }
        var bytes = Data(count: 32)
        let status = bytes.withUnsafeMutableBytes {
            SecRandomCopyBytes(kSecRandomDefault, 32, $0.baseAddress!)
        }
        guard status == errSecSuccess else { throw KeychainError.status(status) }
        try store(bytes)
        return bytes
    }

    static func load() throws -> Data? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        var out: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &out)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess else { throw KeychainError.status(status) }
        return out as? Data
    }

    static func store(_ data: Data) throws {
        let attrs: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            // Disponível em segundo plano após o primeiro desbloqueio, sem sair do aparelho.
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
            kSecValueData as String: data,
        ]
        let status = SecItemAdd(attrs as CFDictionary, nil)
        guard status == errSecSuccess else { throw KeychainError.status(status) }
    }
}

enum KeychainError: Error {
    case status(OSStatus)
}

class RenovadorBgPlugin: Plugin {
    private static var registered = false

    override init() {
        super.init()
        // O registro deve acontecer antes do fim do lançamento do app.
        if !RenovadorBgPlugin.registered {
            RenovadorBgPlugin.registered = BGTaskScheduler.shared.register(
                forTaskWithIdentifier: renewTaskIdentifier, using: nil
            ) { task in
                RenovadorBgPlugin.handle(task: task as! BGAppRefreshTask)
            }
        }
    }

    /// Esqueleto: o ciclo via FFI e a notificação "toque para renovar" ficam para a Onda 3.
    static func handle(task: BGAppRefreshTask) {
        task.expirationHandler = {
            task.setTaskCompleted(success: false)
        }
        task.setTaskCompleted(success: true)
    }

    @objc public func scheduleNext(_ invoke: Invoke) throws {
        let args = try invoke.parseArgs(ScheduleNextArgs.self)
        guard let date = ISO8601DateFormatter().date(from: args.at) else {
            invoke.reject("horário inválido: \(args.at)")
            return
        }
        let request = BGAppRefreshTaskRequest(identifier: renewTaskIdentifier)
        request.earliestBeginDate = max(date, Date())
        do {
            try BGTaskScheduler.shared.submit(request)
            invoke.resolve()
        } catch {
            invoke.reject(error.localizedDescription)
        }
    }

    @objc public func getDbKey(_ invoke: Invoke) throws {
        do {
            let key = try DbKeyKeychain.getOrCreate()
            invoke.resolve(["keyHex": key.map { String(format: "%02x", $0) }.joined()])
        } catch {
            invoke.reject("falha ao ler a chave: \(error)")
        }
    }
}

@_cdecl("init_plugin_renovador_bg")
func initPlugin() -> Plugin {
    return RenovadorBgPlugin()
}
