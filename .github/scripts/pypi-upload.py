"""Upload one distribution to PyPI with twine and report the rate-limit answer.

twine prints PyPI's response body but not its headers, and PyPI reports its
new-project limit, what is left of it, and when it resets only in the RateLimit
and RateLimit-Policy headers it emits alongside a refusal. Reading them is what
lets the caller wait for the window to open instead of guessing at it.

    pypi-upload.py <distribution file>

The arguments and the exit code are twine's own. Only the two rate-limit
headers and Retry-After are printed; nothing from the request is, since that
carries the upload token.
"""

import re
import sys

import requests.adapters
from twine.__main__ import main

#: A policy in the form PyPI emits, from draft-ietf-httpapi-ratelimit-headers-10:
#: what is left of the policy (r) and the seconds until it resets (t).
STATE = re.compile(r'"(?P<name>[^"]+)"\s*;\s*r=(?P<remaining>\d+)\s*;\s*t=(?P<resets>\d+)')

_send = requests.adapters.HTTPAdapter.send


def report(response):
    """Print what a response says about the limits it was measured against.

    :param response: The response twine received from the index.
    """
    policy = response.headers.get("RateLimit-Policy")
    state = response.headers.get("RateLimit")
    retry_after = response.headers.get("Retry-After")
    if policy:
        print("rate limit policy: " + policy, flush=True)
    if retry_after:
        print("retry after: " + retry_after + "s", flush=True)
    if not state:
        return
    print("rate limit state: " + state, flush=True)
    spent = [int(m["resets"]) for m in STATE.finditer(state) if m["remaining"] == "0"]
    if spent:
        print("pypi-reset-seconds: " + str(max(spent)), flush=True)


def send(self, request, **kwargs):
    """Send a request through twine's adapter and report the limits it reports.

    :param request: The prepared request twine is sending.
    :returns: The response, unchanged.
    """
    response = _send(self, request, **kwargs)
    report(response)
    return response


requests.adapters.HTTPAdapter.send = send
# The progress bar writes a carriage return and an erase-line escape without a
# newline, which prefixes whatever is printed next and splits it in the CI log.
sys.argv = [
    "twine",
    "upload",
    "--skip-existing",
    "--non-interactive",
    "--disable-progress-bar",
    "--verbose",
] + sys.argv[1:]
sys.exit(main())
