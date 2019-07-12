pragma solidity ^0.5.0;

contract MyContract {

	// A method to be called from another contract
	
	function remoteMethod(uint value) public {
		msg.sender.transfer(value);
		return; 
	}
	
}
