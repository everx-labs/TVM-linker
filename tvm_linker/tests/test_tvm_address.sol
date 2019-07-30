pragma solidity ^0.5.0;

contract TestThisAddress {
	
	function tvm_address() private returns (address) {}
	
	function main() payable public returns (address) {
		return tvm_address();
	}
	
}
