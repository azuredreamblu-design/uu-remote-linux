param([ValidateRange(-1,3)][int]$ControllerIndex = -1)
$ErrorActionPreference = 'Stop'
if (-not ('DeckPadRumbleTest' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class DeckPadRumbleTest {
    [StructLayout(LayoutKind.Sequential)]
    public struct Pad {
        public ushort buttons;
        public byte lt, rt;
        public short lx, ly, rx, ry;
    }
    [StructLayout(LayoutKind.Sequential)]
    public struct State { public uint packet; public Pad pad; }
    [StructLayout(LayoutKind.Sequential)]
    public struct Vibration { public ushort left, right; }
    [DllImport("xinput1_4.dll", EntryPoint="XInputGetState")]
    private static extern uint Get(uint index, out State state);
    [DllImport("xinput1_4.dll", EntryPoint="XInputSetState")]
    private static extern uint Set(uint index, ref Vibration value);
    public static bool Connected(uint index) { State state; return Get(index, out state)==0; }
    public static uint Rumble(uint index, ushort left, ushort right) {
        var value = new Vibration {left=left, right=right}; return Set(index, ref value);
    }
}
'@
}
Write-Host 'Keep the Deck UU desktop connected; release all controls. / 请保持 Deck 远程连接并松开按键。'
Write-Host 'Starting in 8 seconds. Return focus to the Deck viewer. / 8 秒后开始，请切回 Deck 远程画面。'
Start-Sleep -Seconds 8
$connected = @(0..3 | Where-Object { [DeckPadRumbleTest]::Connected([uint32]$_) })
if ($ControllerIndex -eq -1) {
    if ($connected.Count -ne 1) {
        throw "Expected exactly one XInput controller; detected: $($connected -join ', '). Connect UU first, or select -ControllerIndex 0..3. / 请先连接；多个手柄时指定编号。"
    }
    $ControllerIndex = $connected[0]
}
if ($connected -notcontains $ControllerIndex) { throw 'Selected controller is not connected. / 指定手柄未连接。' }
try {
    foreach ($step in @(
        @{Label='Left / 左侧'; Left=22937; Right=0; Ms=1000},
        @{Label='Stop / 停止'; Left=0; Right=0; Ms=700},
        @{Label='Right / 右侧'; Left=0; Right=22937; Ms=1000},
        @{Label='Stop / 停止'; Left=0; Right=0; Ms=700},
        @{Label='Both / 双侧'; Left=29490; Right=29490; Ms=2000}
    )) {
        Write-Host $step.Label
        $result = [DeckPadRumbleTest]::Rumble([uint32]$ControllerIndex, [uint16]$step.Left, [uint16]$step.Right)
        if ($result -ne 0) { throw "XInputSetState failed: $result" }
        Start-Sleep -Milliseconds $step.Ms
    }
} finally {
    [void][DeckPadRumbleTest]::Rumble([uint32]$ControllerIndex, 0, 0)
}
Write-Host 'Done; rumble stopped. / 测试完成，震动已停止。'
Write-Host 'XInput success alone does not prove Deck vibration; confirm by touch. / 以 Deck 实际震感为准。'
