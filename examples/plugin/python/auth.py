from gung import auth

def authenticate(ctx: auth.AuthContext) -> auth.AuthResp:
    print(ctx.requests[-1].payload)
    if ctx.requests[-1].payload.get("password") == None:
        return auth.AuthResp.challenge("password is required", ["password"])
    elif ctx.requests[-1].payload.get("password") != 123456:
        return auth.AuthResp.reject("password is incorrect")
    else:
        return auth.AuthResp.accept("pass")
