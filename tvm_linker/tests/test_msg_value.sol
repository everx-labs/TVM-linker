pragma solidity ^0.5.0;

contract TestMsgValue {
	
	function main() payable public returns (uint256) {
		return msg.value;
	}
	
}
