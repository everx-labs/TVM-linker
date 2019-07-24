pragma solidity ^0.5.0;

contract TestTvmRandSeed {
	
	function tvm_rand_seed() private returns (uint256) {}
	
	function main() payable public returns (uint256) {
		return tvm_rand_seed();
	}
	
}
