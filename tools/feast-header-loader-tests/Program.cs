using System.Reflection;
using System.Collections;
var path = Path.GetFullPath(args[0]);
var assembly = Assembly.Load(File.ReadAllBytes(Path.Combine(path,"Kawari.FeastHud.dll")));
if (assembly.Location != "") throw new Exception("Must reproduce Dalamud's memory-loaded assembly");
var type = assembly.GetType("Kawari.FeastHud.NativeMatchHeader",true)!;
var header = Activator.CreateInstance(type,new object[]{path});
var atlases = (IDictionary)type.GetField("atlases",BindingFlags.Instance|BindingFlags.NonPublic)!.GetValue(header)!;
if (atlases.Count != 2) throw new Exception("Expected both original atlases");
Console.WriteLine("PASS: memory-loaded assembly has empty Location; explicit plugin path loads both Feast atlases.");
// Compare the projection bounds against the actual SDK payload types, not just
// the manually declared test buffers. Standard/simple share these data types.
var layouts = (IEnumerable)assembly.GetType("Kawari.FeastHud.NativeJobGaugeData",true)!
    .GetField("Layouts",BindingFlags.Static|BindingFlags.NonPublic)!.GetValue(null)!;
var sdk=typeof(FFXIVClientStructs.FFXIV.Client.UI.AddonJobHudWHM0).Assembly;
int layoutChecks=0;
foreach(var layout in layouts)
{
    var lt=layout.GetType();
    string name=(string)lt.GetProperty("Name")!.GetValue(layout)!;
    int size=(int)lt.GetProperty("Size")!.GetValue(layout)!;
    string typeName="FFXIVClientStructs.FFXIV.Client.UI.Addon"+name.Replace("NIN1v70","NIN1");
    var native=sdk.GetType(typeName,true)!;
    var dataType=native.GetField("DataCurrent")!.FieldType;
    int previous=(int)System.Runtime.InteropServices.Marshal.OffsetOf(native,"DataPrevious");
    int current=(int)System.Runtime.InteropServices.Marshal.OffsetOf(native,"DataCurrent");
    if (size>current-previous || size>System.Runtime.InteropServices.Marshal.SizeOf(dataType))
        throw new Exception($"Gauge projection overruns native payload: {name}");
    layoutChecks++;
}
Console.WriteLine($"PASS: {layoutChecks} native gauge payload bounds match SDK layouts.");
