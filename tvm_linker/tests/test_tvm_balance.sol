pragma solidity ^0.5.0;

contract TestNow {
	
	function tvm_balance() private returns (uint16) {}
	
	function main() payable public returns (uint16) {
		return tvm_balance();
	}
	
}
