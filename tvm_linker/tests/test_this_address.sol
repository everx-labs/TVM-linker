pragma solidity ^0.5.0;

contract TestThisAddress {
	
	function main() payable public returns (address) {
		return this.address;
	}
	
}
