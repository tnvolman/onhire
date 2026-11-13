# Security

Onhire is an **unaudited testnet prototype**. Do not use it with real funds.
There is no mainnet deployment and none is planned in this version.

Known, disclosed properties:

- The collection account on testnet keeps the deployer's access key (it is
  also the minter). That key cannot move box tokens or box funds, but could
  redeploy the collection contract.
- Box accounts have zero access keys and no upgrade, deploy, add-key or
  delete path.
- `withdraw` pays a pull balance with a plain transfer. If the caller deletes
  their own account between the call and the transfer receipt, the transfer
  refund lands back on the box as surplus (the invariant still holds).

The threat model is in `README.md` and `docs/ONHIRE-SPEC-v1.1.md` section 7.

To report a problem, contact tnvolman (https://github.com/tnvolman) privately on
GitHub. Please do not open a public issue for an unfixed vulnerability.
