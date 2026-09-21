namespace Kawari.FeastHud;

// Match the server's original animation IDs before local prediction begins.
// Button IDs still identify requests/recasts; spell IDs identify presentation.
public static class FeastSpells
{
    public static uint Resolve(uint id) => id switch
    {
        120 => 8895, 135 => 8896, 124 or 133 or 37010 => 18945,
        16533 or 119 or 127 or 3568 or 7431 or 25859 => 17789,
        16532 or 121 or 132 => 17790,
        16531 => 17791, 16534 => 18946, 16535 => 17793,
        3570 => 13975, 3571 => 9620, 16536 => 17794,
        7568 => 1584, 3 => 18942, 140 => 18943, 16560 => 17690,
        7432 => 17832, 7571 => 17688, 25861 => 18957, 7433 => 17689,
        208 => 3362, _ => id,
    };
}
