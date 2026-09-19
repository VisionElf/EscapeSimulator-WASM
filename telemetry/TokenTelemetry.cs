using System;
using System.Reflection;
using System.Threading;
using BepInEx;
using HarmonyLib;
using UnityEngine;

namespace EscapeSimulator.Telemetry
{
    // Public, non-const fields form the externally read protocol. No save edits or network I/O.
    [BepInPlugin("local.escapesimulator.telemetry", "Escape Simulator Token Telemetry", "0.1.0")]
    public sealed class TokenTelemetry : BaseUnityPlugin
    {
        public static int ProtocolVersion = 1;
        public static bool Ready;
        public static int Sequence;
        public static object GameInstance;
        public static ulong PickupCount;
        private Harmony patches;

        private void Awake()
        {
            try
            {
                Type game = AccessTools.TypeByName("Game");
                if (game == null || game.Assembly.GetName().Name != "EscapeSimulator.Core")
                    throw new MissingMemberException("EscapeSimulator.Core.Game not found");
                MethodInfo init = AccessTools.DeclaredMethod(game, "init", Type.EmptyTypes);
                MethodInfo pickup = AccessTools.DeclaredMethod(game, "handleToken", new[] { typeof(GameObject), typeof(int) });
                if (init == null || pickup == null || init.IsStatic || pickup.IsStatic ||
                    init.ReturnType != typeof(void) || pickup.ReturnType != typeof(void))
                    throw new MissingMethodException("Expected Game.init() and Game.handleToken(GameObject,int)");
                patches = new Harmony("local.escapesimulator.telemetry");
                patches.Patch(init, postfix: new HarmonyMethod(typeof(TokenTelemetry), nameof(RoomInitialized)));
                patches.Patch(pickup, postfix: new HarmonyMethod(typeof(TokenTelemetry), nameof(TokenPickedUp)));
                Ready = true;
                Logger.LogInfo("Token protocol 1 ready. Both Game hooks installed; no game behavior replaced.");
                Logger.LogInfo("Game assembly MVID: " + game.Module.ModuleVersionId);
            }
            catch (Exception error)
            {
                Ready = false;
                patches?.UnpatchSelf();
                Logger.LogError("Token telemetry disabled: " + error);
            }
        }

        private static void RoomInitialized(object __instance)
        {
            Interlocked.Increment(ref Sequence);
            GameInstance = __instance;
            PickupCount = 0;
            Interlocked.Increment(ref Sequence);
        }

        private static void TokenPickedUp(object __instance)
        {
            if (!Ready) return;
            Interlocked.Increment(ref Sequence);
            if (!ReferenceEquals(GameInstance, __instance))
            {
                GameInstance = __instance;
                PickupCount = 0;
            }
            PickupCount++;
            Interlocked.Increment(ref Sequence);
        }

        private void OnDestroy()
        {
            Ready = false;
            patches?.UnpatchSelf();
            GameInstance = null;
        }
    }
}
