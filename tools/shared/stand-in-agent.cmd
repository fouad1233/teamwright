@echo off
rem A stand-in for Claude Code, for the tools that drive the built program.
rem
rem It answers the two questions Teamwright asks the program before it lets
rem a specialist work: which version it is, and whether the user is signed
rem in. It cannot do any work, and it is never asked to: it is a script, and
rem Teamwright does not start a turn on a script. That refusal is what the
rem smoke test looks at.
rem
rem The tools point Teamwright at it with TEAMWRIGHT_AGENT_PROGRAM. What a
rem test or a picture shows then does not depend on the Claude Code of the
rem computer it runs on, and says nothing about how its user is signed in.

if "%~1"=="--version" goto version
if "%~1"=="auth" goto sign_in

echo This is a stand-in for Claude Code. It cannot do any work. 1>&2
exit /b 1

:version
echo 2.1.322 (Claude Code)
exit /b 0

:sign_in
echo {"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty","subscriptionType":"max"}
exit /b 0
