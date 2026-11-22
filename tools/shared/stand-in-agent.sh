#!/bin/sh
# A stand-in for Claude Code, for the tools that drive the built program on
# Linux. It does what stand-in-agent.cmd does on Windows.
#
# It answers the two questions Teamwright asks the program before it lets a
# specialist work: which version it is, and whether the user is signed in.
# It cannot do any work. On Windows Teamwright refuses to start a turn on a
# script; on Linux it does start one, and the stand-in ends it at once with a
# failure. That failure is what the smoke test looks at.
#
# The tools point Teamwright at it with TEAMWRIGHT_AGENT_PROGRAM. What a test
# or a picture shows then does not depend on the Claude Code of the computer
# it runs on, and says nothing about how its user is signed in.

case "$1" in
  --version)
    echo "2.1.322 (Claude Code)"
    exit 0
    ;;
  auth)
    echo '{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty","subscriptionType":"max"}'
    exit 0
    ;;
esac

echo "This is a stand-in for Claude Code. It cannot do any work." 1>&2
exit 1
