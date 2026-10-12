export enum ClaimRole {
    SUPERADMIN = 'superadmin',
    ADMIN = 'admin',
    DEVELOPER = 'developer',
    USER = 'user',
    EMPLOYEE = 'employee',
}

export interface Claim {
    companyKey: string;
    companyCode: string;
    role: string;
    scope: string;
    username: string;
    email: string;
    email_verified: boolean;
    phone: string;
    phone_verified: boolean;
    aud: string;
    exp: number;
    jti: string;
    iat: number;
    iss: string;
    sub: string;
}
