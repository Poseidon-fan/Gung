from typing import Any, List, Dict, Union
from enum import Enum

# JSON value type that corresponds to serde_json::Value in Rust
# Recursive type definition for proper JSON structure typing
JsonValue = Union[Dict[str, "JsonValue"], List["JsonValue"], str, int, float, bool, None]

# Auth module namespace - only this is exposed at the top level
class auth:
    class AuthType(Enum):
        Ping: int
        Connect: int

    class AuthReq:
        @property
        def payload(self) -> JsonValue: ...

    class AuthAcceptResp:
        def __init__(self, msg: str) -> None: ...

    class AuthRejectResp:
        def __init__(self, msg: str) -> None: ...

    class AuthChallengeResp:
        msg: str
        required_fields: List[str]

        def __init__(self, msg: str, required_fields: List[str]) -> None: ...

    class AuthResp(Enum):
        Accept: "auth.AuthAcceptResp"
        Reject: "auth.AuthRejectResp"
        Challenge: "auth.AuthChallengeResp"

    class AuthContext:
        auth_type: "auth.AuthType"
        requests: List["auth.AuthReq"]
        responses: List["auth.AuthResp"]

        @property
        def client_version(self) -> str: ...
