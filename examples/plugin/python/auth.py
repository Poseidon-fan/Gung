from gung import auth

def authenticate(ctx: auth.AuthContext) -> auth.AuthResp:
    print(ctx.client_addr)
    return auth.AuthResp.accept("pass")
