pragma solidity ^0.5.0;

contract TestArray {

    function main(uint8 idx, uint256[] memory myarray) public pure returns (uint256) {
        return myarray[idx];
    }
    
}
