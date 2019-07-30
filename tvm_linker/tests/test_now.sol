pragma solidity ^0.5.0;

contract TestNow {
	
	function tvm_now() private returns (uint32) {}
	
	function main() payable public returns (uint32) {
		return tvm_now();
	}
	
}
