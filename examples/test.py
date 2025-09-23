from gung import auth

acc_resp = auth.AuthAcceptResp("pass")
print(acc_resp)

challenge_resp = auth.AuthChallengeResp("challenge resp!", ["username", "password"])
print(challenge_resp.msg)
print(challenge_resp.required_fields)
