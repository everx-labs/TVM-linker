pragma solidity ^0.5.0;

contract TestTvmBlockLt {
	
	function tvm_block_lt() private returns (uint64) {}
	
	function main() payable public returns (uint64) {
		return tvm_block_lt();
	}
	
}
