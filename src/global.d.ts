declare module 'jshashes' {
    export class SHA256 {
        b64(input: string): string;
        b64_hmac(key: string, input: string): string;
    }
}
