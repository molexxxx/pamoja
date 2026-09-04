using Pamoja.Core;
using Pamoja.Native.Interop;

namespace Pamoja.Routing;

/// <summary>What to do with a packet bound for a given node.</summary>
public enum ForwardAction
{
    /// <summary>The packet is for this node; hand it to the application.</summary>
    Deliver = 0,

    /// <summary>A route is known; unicast the packet to the next hop reported alongside.</summary>
    Relay = 1,

    /// <summary>No route is known; fall back to flooding the packet.</summary>
    Flood = 2,
}

/// <summary>A routing decision, and the neighbour it names when there is one.</summary>
/// <param name="Action">What to do with the packet.</param>
/// <param name="NextHop">
/// The neighbour to unicast to, or <c>null</c> unless the action is
/// <see cref="ForwardAction.Relay"/>.
/// </param>
public readonly record struct ForwardDecision(ForwardAction Action, uint? NextHop);

/// <summary>A learned way to reach one node.</summary>
/// <param name="Dst">The node this route reaches.</param>
/// <param name="NextHop">The neighbour to send a packet to on the way there.</param>
/// <param name="Cost">What the route costs, usually in hops.</param>
public readonly record struct Route(uint Dst, uint NextHop, ushort Cost);

/// <summary>One node routing table, learned from the traffic the node hears.</summary>
/// <remarks>
/// Flooding always works but costs every node airtime and power on every packet.
/// A node that remembers the way can forward to one neighbour instead, and falls
/// back to flooding rather than failing whenever it does not know the way. The
/// core table is generic over its size, which cannot cross the C ABI, so this one
/// is sized when it is built.
/// </remarks>
public sealed class Router : IDisposable
{
    private readonly NativeHandle _handle;

    /// <summary>Creates an empty routing table for a node.</summary>
    /// <param name="address">
    /// The address of this node, which is what a routing decision recognises as a
    /// local delivery.
    /// </param>
    /// <param name="capacity">
    /// How many routes to make room for. A capacity of 0 floods every unknown
    /// destination, which is the behaviour with no table at all.
    /// </param>
    /// <exception cref="PamojaException">The native table could not be created.</exception>
    public Router(uint address, int capacity = NativeMethods.RoutingDefaultCapacity)
    {
        _handle = NativeHandle.Create(
            NativeMethods.pamoja_router_new(address, (nuint)capacity),
            NativeMethods.pamoja_router_free,
            "router");
    }

    /// <summary>The address this router answers for.</summary>
    public uint Address => _handle.Use(NativeMethods.pamoja_router_address);

    /// <summary>How many routes the table currently holds.</summary>
    public int Count => checked((int)_handle.Use(NativeMethods.pamoja_router_len));

    /// <summary>How many routes the table can hold.</summary>
    public int Capacity =>
        checked((int)_handle.Use(NativeMethods.pamoja_router_capacity));

    /// <summary>Learns a route from a packet that arrived.</summary>
    /// <param name="origin">The node the packet came from.</param>
    /// <param name="via">The neighbour it arrived through.</param>
    /// <param name="cost">What that path costs, usually a hop count.</param>
    /// <returns>
    /// Whether the table changed. It keeps the cheapest way it knows to each node,
    /// and when full gives up the most expensive route to make room for a cheaper
    /// one.
    /// </returns>
    public bool Observe(uint origin, uint via, ushort cost) =>
        _handle.Use(handle => NativeMethods.pamoja_router_observe(handle, origin, via, cost));

    /// <summary>Returns the neighbour on the way to a node.</summary>
    /// <param name="dst">The node to reach.</param>
    /// <returns>The next hop, or <c>null</c> when no route is known.</returns>
    public uint? NextHop(uint dst) =>
        _handle.Use(handle =>
            NativeMethods.pamoja_router_next_hop(handle, dst, out uint next) ? next : (uint?)null);

    /// <summary>Returns what the known route to a node costs.</summary>
    /// <param name="dst">The node to reach.</param>
    /// <returns>The cost, or <c>null</c> when no route is known.</returns>
    public ushort? Cost(uint dst) =>
        _handle.Use(handle =>
            NativeMethods.pamoja_router_cost(handle, dst, out ushort cost) ? cost : (ushort?)null);

    /// <summary>Returns the whole route to a node.</summary>
    /// <param name="dst">The node to reach.</param>
    /// <returns>The route, or <c>null</c> when no route is known.</returns>
    public Route? RouteTo(uint dst) =>
        _handle.Use(handle =>
            NativeMethods.pamoja_router_route(handle, dst, out PamojaRoute route)
                ? new Route(route.Dst, route.NextHop, route.Cost)
                : (Route?)null);

    /// <summary>Decides what to do with a packet bound for a node.</summary>
    /// <param name="dst">The node the packet is addressed to.</param>
    /// <returns>The decision, carrying a next hop only when it says to relay.</returns>
    public ForwardDecision Forward(uint dst) =>
        _handle.Use(handle =>
        {
            PamojaForward action = NativeMethods.pamoja_router_forward(handle, dst, out uint next);
            return action == PamojaForward.Relay
                ? new ForwardDecision(ForwardAction.Relay, next)
                : new ForwardDecision((ForwardAction)action, null);
        });

    /// <summary>Forgets the route to a node, for example after it stops answering.</summary>
    /// <param name="dst">The node to forget.</param>
    public void Forget(uint dst) =>
        _handle.Use(handle => NativeMethods.pamoja_router_forget(handle, dst));

    /// <inheritdoc/>
    public void Dispose() => _handle.Dispose();
}
