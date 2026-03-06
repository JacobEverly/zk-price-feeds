const providers = [
  { name: "Chainlink", provingCost: "$0 (LINK-subsidized)", trust: "4-31 node committee", gas: "~400K", color: "text-chainlink-blue", barWidth: "100%", barColor: "bg-chainlink-blue" },
  { name: "Pyth", provingCost: "$0.001/update", trust: "Wormhole guardians", gas: "~65K", color: "text-pyth-purple", barWidth: "40%", barColor: "bg-pyth-purple" },
  { name: "RedStone", provingCost: "$0 (sponsor-paid)", trust: "Signer set", gas: "~90K", color: "text-redstone-red", barWidth: "35%", barColor: "bg-redstone-red" },
  { name: "Chronicle", provingCost: "$0 (sponsor-paid)", trust: "Schnorr multisig", gas: "~150K", color: "text-chronicle-teal", barWidth: "30%", barColor: "bg-chronicle-teal" },
  { name: "ZK Oracle", provingCost: "$0.004 + gas", trust: "Mathematics", gas: "~250K", color: "text-neon-green", barWidth: "3%", barColor: "bg-gradient-to-r from-neon-green to-neon-cyan", highlight: true },
];

const l2Rows = [
  { freq: "Every 1 min", l1: "$145,818", l2: "$450" },
  { freq: "Every 5 min", l1: "$29,164", l2: "$90" },
  { freq: "Every 1 hour", l1: "$2,430", l2: "$7.50" },
  { freq: "Every 24 hours", l1: "$101", l2: "$0.31" },
];

export default function CostComparison() {
  return (
    <section id="cost" className="py-28 px-6 relative overflow-hidden">
      <div className="absolute bottom-0 left-1/2 -translate-x-1/2 w-[600px] h-[400px] bg-neon-purple/5 rounded-full blur-[200px]" />

      <div className="max-w-6xl mx-auto relative z-10">
        <div className="text-center mb-16">
          <h2 className="text-4xl sm:text-6xl font-display tracking-wide">
            <span className="text-text-primary block">The real cost of a price feed,</span>
            <span className="gradient-text-cyan-purple block text-3xl sm:text-5xl mt-2">compared.</span>
          </h2>
        </div>

        {/* Bar chart */}
        <div className="mb-16 space-y-5 max-w-3xl mx-auto">
          <p className="text-xs font-mono text-text-muted tracking-widest mb-6">
            COST TO GET A FEED
          </p>
          {providers.map((p) => (
            <div key={p.name} className="flex items-center gap-4">
              <div className={`w-24 text-right text-sm font-bold font-body ${p.color} shrink-0`}>{p.name}</div>
              <div className="flex-1 h-10 bg-white/5 rounded-xl overflow-hidden">
                <div
                  className={`h-full ${p.barColor} rounded-xl flex items-center justify-end px-4 transition-all duration-1000`}
                  style={{ width: p.barWidth, minWidth: p.highlight ? "90px" : undefined }}
                >
                  {p.highlight && <span className="text-bg text-xs font-bold font-mono whitespace-nowrap">$0.01/update</span>}
                </div>
              </div>
            </div>
          ))}
          <p className="text-xs text-text-muted font-body">
            ZK Oracle: $0.004 per update + gas. No annual contract.
          </p>
        </div>

        {/* Provider comparison table */}
        <div className="glass rounded-2xl overflow-hidden mb-16">
          <table className="w-full">
            <thead>
              <tr className="border-b border-white/10">
                <th className="text-left py-4 px-5 text-xs font-mono text-text-muted tracking-widest">PROVIDER</th>
                <th className="text-left py-4 px-5 text-xs font-mono text-text-muted tracking-widest">UPDATE COST</th>
                <th className="text-left py-4 px-5 text-xs font-mono text-text-muted tracking-widest">ON-CHAIN GAS</th>
                <th className="text-left py-4 px-5 text-xs font-mono text-text-muted tracking-widest">TRUST</th>
              </tr>
            </thead>
            <tbody>
              {providers.map((p) => (
                <tr
                  key={p.name}
                  className={`border-b border-white/5 ${p.highlight ? "" : "hover:bg-white/[0.03]"} transition-colors`}
                  style={p.highlight ? { background: "linear-gradient(135deg, rgba(57,255,20,0.05), rgba(0,229,255,0.03))" } : undefined}
                >
                  <td className={`py-4 px-5 font-bold font-body ${p.color}`}>{p.name}</td>
                  <td className="py-4 px-5 text-sm font-mono">
                    <span className={p.highlight ? "text-neon-green font-bold" : "text-text-secondary"}>{p.provingCost}</span>
                  </td>
                  <td className="py-4 px-5 text-sm font-mono">
                    <span className={p.highlight ? "text-neon-green font-bold" : "text-text-secondary"}>{p.gas}</span>
                  </td>
                  <td className="py-4 px-5 text-sm">
                    {p.highlight ? (
                      <span className="inline-flex items-center px-3 py-1.5 rounded-full bg-neon-green/10 border border-neon-green/30 text-neon-green font-bold font-mono text-xs tracking-wider">{p.trust}</span>
                    ) : (
                      <span className="text-text-muted font-body">{p.trust}</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Verification cost callout */}
        <div className="glass-strong rounded-2xl p-8 max-w-3xl mx-auto text-center mb-16">
          <p className="text-text-secondary font-body text-lg">
            Chainlink verifies BLS aggregate signatures on-chain at{" "}
            <span className="text-chainlink-blue font-mono font-bold">~400K gas</span>.
            Chronicle uses Schnorr multisigs at{" "}
            <span className="text-chronicle-teal font-mono font-bold">~150K gas</span>.
            A ZK proof verifies at{" "}
            <span className="text-neon-green font-mono font-bold">~250K gas</span>{" "}
            -- cheaper than BLS, with no committee to trust.
          </p>
        </div>

        {/* L2 Economics -- absorbed */}
        <div className="text-center mb-10">
          <h3 className="text-3xl sm:text-4xl font-display tracking-wide">
            <span className="text-text-primary">On L2, this gets </span>
            <span className="text-neon-cyan">ridiculous.</span>
          </h3>
        </div>

        <div className="glass rounded-2xl overflow-hidden mb-8 max-w-3xl mx-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-white/10">
                <th className="text-left py-4 px-6 text-xs font-mono text-text-muted tracking-widest">FREQUENCY</th>
                <th className="text-right py-4 px-6 text-xs font-mono text-text-muted tracking-widest">L1 / MONTH</th>
                <th className="text-right py-4 px-6 text-xs font-mono text-text-muted tracking-widest">L2 / MONTH</th>
              </tr>
            </thead>
            <tbody>
              {l2Rows.map((row) => (
                <tr key={row.freq} className="border-b border-white/5 hover:bg-white/[0.03] transition-colors">
                  <td className="py-4 px-6 font-medium text-text-primary font-body">{row.freq}</td>
                  <td className="py-4 px-6 text-right font-mono text-text-muted line-through decoration-neon-pink/60 decoration-2">{row.l1}</td>
                  <td className="py-4 px-6 text-right font-mono font-bold text-neon-green text-lg">{row.l2}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Callout */}
        <div className="glass-strong rounded-2xl p-8 text-center max-w-3xl mx-auto glow-green">
          <p className="text-xl font-display tracking-wide text-neon-green">
            A continuously-updated L2 feed costs less than a cup of coffee per month.
          </p>
        </div>
      </div>
    </section>
  );
}
